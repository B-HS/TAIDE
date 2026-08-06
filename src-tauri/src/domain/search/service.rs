use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use regex::{Regex, RegexBuilder};

use crate::constants;
use crate::domain::file::service as file_service;
use crate::error::{AppError, AppResult};
use crate::infra::persist;

use super::types::{SearchMatch, SearchQuery, SearchReplaceResult, SEARCH_MATCH_LIMIT};

const BINARY_SNIFF_BYTES: usize = 8_000;
const PREVIEW_TRUNCATE_THRESHOLD_BYTES: usize = 200;
const PREVIEW_CONTEXT_BYTES: usize = 60;
const PREVIEW_ELLIPSIS: &str = "…";

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_whole_word_match(haystack: &str, start: usize, end: usize) -> bool {
    let before_ok = haystack[..start].chars().next_back().map(|c| !is_word_char(c)).unwrap_or(true);
    let after_ok = haystack[end..].chars().next().map(|c| !is_word_char(c)).unwrap_or(true);
    before_ok && after_ok
}

pub fn find_matches_in_line(line: &str, query: &SearchQuery) -> Vec<(u32, u32)> {
    if query.text.is_empty() {
        return Vec::new();
    }

    let haystack = if query.case_sensitive {
        line.to_string()
    } else {
        line.to_ascii_lowercase()
    };
    let needle = if query.case_sensitive {
        query.text.clone()
    } else {
        query.text.to_ascii_lowercase()
    };

    let mut matches = Vec::new();
    let mut search_from = 0usize;

    while search_from <= haystack.len() {
        let Some(relative_index) = haystack[search_from..].find(&needle) else {
            break;
        };
        let start = search_from + relative_index;
        let end = start + needle.len();

        if !query.whole_word || is_whole_word_match(&haystack, start, end) {
            matches.push((start as u32, end as u32));
        }

        search_from = start + needle.len().max(1);
    }

    matches
}

fn compile_regex(query: &SearchQuery) -> AppResult<Regex> {
    RegexBuilder::new(&query.text)
        .case_insensitive(!query.case_sensitive)
        .build()
        .map_err(|error| AppError::InvalidArgument(format!("잘못된 정규식입니다: {error}")))
}

fn find_regex_matches_in_line(line: &str, regex: &Regex, whole_word: bool) -> Vec<(u32, u32)> {
    regex
        .find_iter(line)
        .filter(|found| !whole_word || is_whole_word_match(line, found.start(), found.end()))
        .map(|found| (found.start() as u32, found.end() as u32))
        .collect()
}

enum MatchMode<'a> {
    Literal,
    Regex(&'a Regex),
}

fn matches_for_line(line: &str, query: &SearchQuery, mode: &MatchMode<'_>) -> Vec<(u32, u32)> {
    match mode {
        MatchMode::Literal => find_matches_in_line(line, query),
        MatchMode::Regex(regex) => find_regex_matches_in_line(line, regex, query.whole_word),
    }
}

fn char_boundary_at_or_before(s: &str, byte_index: usize) -> usize {
    let mut index = byte_index.min(s.len());
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn char_boundary_at_or_after(s: &str, byte_index: usize) -> usize {
    let mut index = byte_index.min(s.len());
    while index < s.len() && !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

pub fn build_preview(line: &str, match_start: u32, match_end: u32) -> (String, u32, u32) {
    if line.len() <= PREVIEW_TRUNCATE_THRESHOLD_BYTES {
        return (line.to_string(), match_start, match_end);
    }

    let match_start = match_start as usize;
    let match_end = match_end as usize;

    let window_start = char_boundary_at_or_before(line, match_start.saturating_sub(PREVIEW_CONTEXT_BYTES));
    let window_end = char_boundary_at_or_after(line, (match_end + PREVIEW_CONTEXT_BYTES).min(line.len()));

    let prefix = if window_start > 0 { PREVIEW_ELLIPSIS } else { "" };
    let suffix = if window_end < line.len() { PREVIEW_ELLIPSIS } else { "" };

    let preview = format!("{prefix}{}{suffix}", &line[window_start..window_end]);
    let new_start = (match_start - window_start) + prefix.len();
    let new_end = (match_end - window_start) + prefix.len();

    (preview, new_start as u32, new_end as u32)
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_bytes = pattern.as_bytes();
    let text_bytes = text.as_bytes();

    let mut p = 0usize;
    let mut t = 0usize;
    let mut star_p: Option<usize> = None;
    let mut star_t = 0usize;

    while t < text_bytes.len() {
        if p < pattern_bytes.len() && pattern_bytes[p] == text_bytes[t] {
            p += 1;
            t += 1;
        } else if p < pattern_bytes.len() && pattern_bytes[p] == b'*' {
            star_p = Some(p);
            star_t = t;
            p += 1;
        } else if let Some(sp) = star_p {
            p = sp + 1;
            star_t += 1;
            t = star_t;
        } else {
            return false;
        }
    }

    while p < pattern_bytes.len() && pattern_bytes[p] == b'*' {
        p += 1;
    }

    p == pattern_bytes.len()
}

fn relative_path_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn passes_glob_filters(root: &Path, path: &Path, query: &SearchQuery) -> bool {
    let relative = relative_path_string(root, path);

    if let Some(include) = &query.include_glob {
        if !glob_match(include, &relative) {
            return false;
        }
    }

    if let Some(exclude) = &query.exclude_glob {
        if glob_match(exclude, &relative) {
            return false;
        }
    }

    true
}

fn search_file(
    path: &Path,
    query: &SearchQuery,
    mode: &MatchMode<'_>,
    remaining: u32,
    on_match: &mut impl FnMut(SearchMatch),
) -> AppResult<u32> {
    if remaining == 0 {
        return Ok(0);
    }

    let Ok(bytes) = std::fs::read(path) else {
        return Ok(0);
    };

    let sniff_len = bytes.len().min(BINARY_SNIFF_BYTES);
    if is_binary(&bytes[..sniff_len]) {
        return Ok(0);
    }

    let text = String::from_utf8_lossy(&bytes);
    let path_string = path.to_string_lossy().to_string();
    let mut emitted = 0u32;

    'lines: for (line_index, line) in text.lines().enumerate() {
        for (match_start, match_end) in matches_for_line(line, query, mode) {
            if emitted >= remaining {
                break 'lines;
            }

            let (preview, preview_start, preview_end) = build_preview(line, match_start, match_end);
            on_match(SearchMatch {
                path: path_string.clone(),
                line: (line_index + 1) as u32,
                column: match_start,
                preview,
                match_start: preview_start,
                match_end: preview_end,
            });
            emitted += 1;
        }
    }

    Ok(emitted)
}

pub fn search(root: &Path, query: &SearchQuery, cancelled: &AtomicBool, mut on_match: impl FnMut(SearchMatch)) -> AppResult<u32> {
    if query.text.is_empty() {
        return Ok(0);
    }

    let compiled_regex = if query.regex { Some(compile_regex(query)?) } else { None };
    let mode = match &compiled_regex {
        Some(regex) => MatchMode::Regex(regex),
        None => MatchMode::Literal,
    };

    let mut total = 0u32;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    'walk: while let Some(dir) = stack.pop() {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }

        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            if cancelled.load(Ordering::Relaxed) || total >= SEARCH_MATCH_LIMIT {
                break 'walk;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let path = entry.path();

            if metadata.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !constants::is_ignored_dir(&name) {
                    stack.push(path);
                }
                continue;
            }

            if !metadata.is_file() || !passes_glob_filters(root, &path, query) {
                continue;
            }

            let remaining = SEARCH_MATCH_LIMIT - total;
            total += search_file(&path, query, &mode, remaining, &mut on_match)?;
        }
    }

    Ok(total)
}

fn collect_project_files(root: &Path, query: &SearchQuery) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let path = entry.path();

            if metadata.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !constants::is_ignored_dir(&name) {
                    stack.push(path);
                }
                continue;
            }

            if metadata.is_file() && passes_glob_filters(root, &path, query) {
                files.push(path);
            }
        }
    }

    files
}

fn strip_line_terminator(chunk: &str) -> &str {
    chunk.strip_suffix("\r\n").or_else(|| chunk.strip_suffix('\n')).unwrap_or(chunk)
}

fn apply_line_replacements(chunk: &str, matches: &[(u32, u32)], replacement: &str, output: &mut String) {
    let mut cursor = 0usize;

    for &(start, end) in matches {
        output.push_str(&chunk[cursor..start as usize]);
        output.push_str(replacement);
        cursor = end as usize;
    }

    output.push_str(&chunk[cursor..]);
}

fn replace_in_file(path: &Path, query: &SearchQuery, mode: &MatchMode<'_>, replacement: &str) -> Option<u32> {
    let bytes = std::fs::read(path).ok()?;

    let sniff_len = bytes.len().min(BINARY_SNIFF_BYTES);
    if is_binary(&bytes[..sniff_len]) {
        return None;
    }

    let text = std::str::from_utf8(&bytes).ok()?;
    let mut output = String::with_capacity(text.len());
    let mut file_matches = 0u32;

    for chunk in text.split_inclusive('\n') {
        let line = strip_line_terminator(chunk);
        let line_matches = matches_for_line(line, query, mode);

        if line_matches.is_empty() {
            output.push_str(chunk);
            continue;
        }

        file_matches += line_matches.len() as u32;
        apply_line_replacements(chunk, &line_matches, replacement, &mut output);
    }

    if file_matches == 0 {
        return None;
    }

    persist::write_atomic(path, output.as_bytes()).ok()?;
    Some(file_matches)
}

pub fn replace(root: &Path, query: &SearchQuery, replacement: &str, paths: Option<&[PathBuf]>) -> AppResult<SearchReplaceResult> {
    if query.text.is_empty() {
        return Ok(SearchReplaceResult {
            changed_files: 0,
            replaced_matches: 0,
        });
    }

    let compiled_regex = if query.regex { Some(compile_regex(query)?) } else { None };
    let mode = match &compiled_regex {
        Some(regex) => MatchMode::Regex(regex),
        None => MatchMode::Literal,
    };

    let target_files = match paths {
        Some(explicit) => explicit
            .iter()
            .filter_map(|path| file_service::ensure_within_root(root, path).ok())
            .collect::<Vec<_>>(),
        None => collect_project_files(root, query),
    };

    let mut changed_files = 0u32;
    let mut replaced_matches = 0u32;

    for path in target_files {
        if let Some(count) = replace_in_file(&path, query, &mode, replacement) {
            changed_files += 1;
            replaced_matches += count;
        }
    }

    Ok(SearchReplaceResult {
        changed_files,
        replaced_matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(text: &str) -> SearchQuery {
        SearchQuery {
            text: text.to_string(),
            case_sensitive: false,
            whole_word: false,
            regex: false,
            include_glob: None,
            exclude_glob: None,
        }
    }

    #[test]
    fn 쿼리가_비어있으면_빈_결과를_반환한다() {
        let matches = find_matches_in_line("hello world", &query(""));
        assert!(matches.is_empty());
    }

    #[test]
    fn 대소문자를_구분하지_않으면_모든_대소문자_조합을_찾는다() {
        let matches = find_matches_in_line("Hello hello HELLO", &query("hello"));
        assert_eq!(matches, vec![(0, 5), (6, 11), (12, 17)]);
    }

    #[test]
    fn 대소문자_구분_옵션이면_정확히_일치하는_경우만_찾는다() {
        let mut q = query("hello");
        q.case_sensitive = true;
        let matches = find_matches_in_line("Hello hello HELLO", &q);
        assert_eq!(matches, vec![(6, 11)]);
    }

    #[test]
    fn 단어_단위_옵션이면_부분_단어는_제외한다() {
        let mut q = query("cat");
        q.whole_word = true;
        let matches = find_matches_in_line("cat concatenate cat_food cat", &q);
        assert_eq!(matches, vec![(0, 3), (25, 28)]);
    }

    #[test]
    fn 겹치지_않는_연속_매치를_모두_찾는다() {
        let matches = find_matches_in_line("aaaa", &query("aa"));
        assert_eq!(matches, vec![(0, 2), (2, 4)]);
    }

    #[test]
    fn 짧은_줄은_그대로_미리보기로_반환한다() {
        let (preview, start, end) = build_preview("short line with match", 12, 17);
        assert_eq!(preview, "short line with match");
        assert_eq!((start, end), (12, 17));
    }

    #[test]
    fn 긴_줄은_매치_주변만_잘라내고_말줄임표를_붙인다() {
        let padding = "x".repeat(300);
        let line = format!("{padding}needle{padding}");
        let match_start = padding.len() as u32;
        let match_end = match_start + "needle".len() as u32;

        let (preview, new_start, new_end) = build_preview(&line, match_start, match_end);

        assert!(preview.starts_with('…'));
        assert!(preview.ends_with('…'));
        assert_eq!(&preview[new_start as usize..new_end as usize], "needle");
        assert!(preview.len() < line.len());
    }

    #[test]
    fn 글롭_패턴은_와일드카드로_임의의_문자열에_매칭한다() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("src/*.ts", "src/index.ts"));
        assert!(!glob_match("*.rs", "main.ts"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact.txt", "exact.txt"));
        assert!(!glob_match("exact.txt", "not-exact.txt"));
    }

    #[test]
    fn nul_바이트가_있으면_바이너리로_판별한다() {
        assert!(is_binary(&[0x41, 0x42, 0x00, 0x43]));
        assert!(!is_binary(&[0x41, 0x42, 0x43]));
    }

    #[test]
    fn 유효한_정규식으로_매치를_찾는다() {
        let regex = RegexBuilder::new("n.+dle").case_insensitive(false).build().unwrap();
        let matches = find_regex_matches_in_line("a needle in a haystack", &regex, false);
        assert_eq!(matches, vec![(2, 8)]);
    }

    #[test]
    fn 정규식_매치도_단어_경계_옵션을_적용한다() {
        let regex = RegexBuilder::new("cat").case_insensitive(true).build().unwrap();
        let matches = find_regex_matches_in_line("cat concatenate cat_food cat", &regex, true);
        assert_eq!(matches, vec![(0, 3), (25, 28)]);
    }

    #[test]
    fn case_sensitive_옵션이_정규식_대소문자_구분에_반영된다() {
        let mut q = query("HELLO");
        q.regex = true;
        q.case_sensitive = true;
        let regex = compile_regex(&q).unwrap();
        assert!(!regex.is_match("hello"));
        assert!(regex.is_match("HELLO"));
    }

    #[test]
    fn case_sensitive가_아니면_정규식도_대소문자를_구분하지_않는다() {
        let mut q = query("HELLO");
        q.regex = true;
        let regex = compile_regex(&q).unwrap();
        assert!(regex.is_match("hello"));
    }

    #[test]
    fn 잘못된_정규식은_패닉_대신_에러를_반환한다() {
        let mut q = query("(unterminated");
        q.regex = true;
        let cancelled = AtomicBool::new(false);
        let result = search(Path::new("."), &q, &cancelled, |_| {});
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn 리터럴_검색은_정규식_경로와_무관하게_그대로_동작한다() {
        let matches = find_matches_in_line("aaaa", &query("aa"));
        assert_eq!(matches, vec![(0, 2), (2, 4)]);
    }

    struct Fixture {
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }

    fn build_fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("taide-search-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("src").join("main.rs"), "fn main() {\n    println!(\"needle here\");\n}\n").unwrap();
        std::fs::write(root.join("node_modules").join("ignored.rs"), "needle").unwrap();
        std::fs::write(root.join("binary.bin"), [0x00u8, 0x01, 0x02, b'n', b'e', b'e', b'd', b'l', b'e']).unwrap();
        Fixture { root }
    }

    #[test]
    fn 무시_디렉토리와_바이너리_파일은_검색에서_제외된다() {
        let fixture = build_fixture();
        let mut results = Vec::new();
        let cancelled = AtomicBool::new(false);

        let total = search(&fixture.root, &query("needle"), &cancelled, |item| results.push(item)).unwrap();

        assert_eq!(total, 1);
        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("main.rs"));
        assert_eq!(results[0].line, 2);
    }

    #[test]
    fn 파일_트리에서_정규식_검색이_동작한다() {
        let fixture = build_fixture();
        let mut q = query("n.+dle");
        q.regex = true;
        let cancelled = AtomicBool::new(false);
        let mut results = Vec::new();

        let total = search(&fixture.root, &q, &cancelled, |item| results.push(item)).unwrap();

        assert_eq!(total, 1);
        assert!(results[0].path.ends_with("main.rs"));
        assert_eq!(results[0].line, 2);
    }

    #[test]
    fn 취소_플래그가_설정되면_검색을_중단한다() {
        let fixture = build_fixture();
        let cancelled = AtomicBool::new(true);
        let mut results = Vec::new();

        let total = search(&fixture.root, &query("needle"), &cancelled, |item| results.push(item)).unwrap();

        assert_eq!(total, 0);
        assert!(results.is_empty());
    }

    fn replace_temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("taide-replace-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn 단순_치환은_모든_일치를_바꾸고_파일에_반영한다() {
        let root = replace_temp_root("simple");
        let file = root.join("a.txt");
        std::fs::write(&file, "needle one\nneedle two\n").unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 2);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "found one\nfound two\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 정규식_치환이_동작한다() {
        let root = replace_temp_root("regex");
        let file = root.join("a.txt");
        std::fs::write(&file, "cat1 cat2 dog\n").unwrap();

        let mut q = query("cat[0-9]");
        q.regex = true;
        let result = replace(&root, &q, "pet", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 2);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "pet pet dog\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn case_sensitive_옵션이_치환_대상을_구분한다() {
        let root = replace_temp_root("case-sensitive");
        let file = root.join("a.txt");
        std::fs::write(&file, "Cat cat CAT\n").unwrap();

        let mut q = query("cat");
        q.case_sensitive = true;
        let result = replace(&root, &q, "dog", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 1);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "Cat dog CAT\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 일치하는_항목이_없으면_0을_반환하고_파일을_바꾸지_않는다() {
        let root = replace_temp_root("no-match");
        let file = root.join("a.txt");
        std::fs::write(&file, "no match here\n").unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.changed_files, 0);
        assert_eq!(result.replaced_matches, 0);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "no match here\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 글롭_필터가_치환_대상_파일을_제한한다() {
        let root = replace_temp_root("glob");
        let included = root.join("keep.rs");
        let excluded = root.join("skip.txt");
        std::fs::write(&included, "needle\n").unwrap();
        std::fs::write(&excluded, "needle\n").unwrap();

        let mut q = query("needle");
        q.include_glob = Some("*.rs".to_string());
        let result = replace(&root, &q, "found", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 1);
        assert_eq!(std::fs::read_to_string(&included).unwrap(), "found\n");
        assert_eq!(std::fs::read_to_string(&excluded).unwrap(), "needle\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn paths_가_주어지면_해당_파일만_치환한다() {
        let root = replace_temp_root("explicit-paths");
        let target = root.join("target.txt");
        let other = root.join("other.txt");
        std::fs::write(&target, "needle\n").unwrap();
        std::fs::write(&other, "needle\n").unwrap();

        let result = replace(&root, &query("needle"), "found", Some(std::slice::from_ref(&target))).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 1);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "found\n");
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "needle\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 바이너리_파일은_치환하지_않는다() {
        let root = replace_temp_root("binary");
        let file = root.join("binary.bin");
        std::fs::write(&file, [0x00u8, 0x01, b'n', b'e', b'e', b'd', b'l', b'e']).unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.changed_files, 0);
        assert_eq!(result.replaced_matches, 0);

        std::fs::remove_dir_all(&root).ok();
    }
}
