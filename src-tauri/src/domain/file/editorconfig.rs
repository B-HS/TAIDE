//! `.editorconfig` resolution for a single file — the ancestor chain walk, the INI-ish parse, and
//! the glob matcher section headers are matched with.
//!
//! Scope is deliberately the five *core* properties (`indent_style`, `indent_size`, `tab_width`,
//! `insert_final_newline`, `trim_trailing_whitespace`); `charset` and `end_of_line` are carried in
//! the d-53 contract §5 rather than parsed here, because neither has a place to land yet (the file
//! domain decodes as UTF-8 only, and `file_save` writes back exactly the `String` it is handed).
//!
//! The glob matcher is written here rather than delegated to `globset` (already in the dependency
//! tree via `ignore`) because EditorConfig's pattern language is not `globset`'s: `{num1..num2}`
//! integer ranges have no `globset` equivalent, a brace group without a comma is a *literal*
//! (`{word}` matches the four characters `{`, `w`… ), and `**` may appear inside a path component
//! (`a**z`), which `globset` rejects outright as a malformed recursive wildcard. Translating
//! EditorConfig patterns into `globset` syntax would therefore be its own bug-prone layer with the
//! same test surface as this matcher, plus an impedance mismatch on the cases it cannot express.
//! No crate was added either way — see the contract's §3 record.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use super::types::{EditorConfigIndentStyle, EditorConfigOptions};

const EDITORCONFIG_FILE_NAME: &str = ".editorconfig";

/// How many ancestor directories the chain walk may visit before giving up. The walk normally ends
/// at a `root = true` file or at the filesystem root long before this; the cap only bounds the cost
/// of a pathologically deep path.
const MAX_ANCESTOR_DIRECTORIES: usize = 64;

/// Largest `.editorconfig` this module will read. A config file is a handful of lines in practice —
/// anything past this is not one, and reading it would only be a way to spend memory on a file the
/// user never asked to open.
const MAX_EDITORCONFIG_BYTES: u64 = 64 * 1024;

/// Upper bound accepted for `indent_size`/`tab_width`. EditorConfig itself has no ceiling, but the
/// value goes straight into monaco's model options, so a hostile or fat-fingered config must not be
/// able to hand the editor an absurd tab width. Values outside `1..=MAX` are ignored (left `None`),
/// the same "drop what cannot be honored" rule `settings::service::sanitize_editor_rulers` uses.
const MAX_INDENT_COLUMNS: u32 = 64;

/// Longest section header this module will even attempt to match. What bounds the *cost* of a
/// match is [`glob_matches`]'s memo table (one entry per node/input-position pair), not this
/// constant; this one bounds how many nodes a single pattern can contribute to that table, and
/// keeps a `.editorconfig` from spending memory on a header no human wrote. Real section headers
/// are a dozen characters.
const MAX_SECTION_PATTERN_CHARS: usize = 1024;

/// The only properties this module keeps. Anything else in the file is parsed past and discarded —
/// an unknown property is not an error in EditorConfig, it is simply not ours.
const CORE_PROPERTY_KEYS: [&str; 5] = [
    "indent_style",
    "indent_size",
    "tab_width",
    "insert_final_newline",
    "trim_trailing_whitespace",
];

/// EditorConfig's "forget whatever an outer file said about this property" value.
const UNSET_VALUE: &str = "unset";

/// `indent_size = tab` means "however wide a tab is here", i.e. defer to `tab_width`.
const INDENT_SIZE_TAB_VALUE: &str = "tab";

const ROOT_KEY: &str = "root";
const TRUE_VALUE: &str = "true";
const FALSE_VALUE: &str = "false";

#[derive(Debug, Default, PartialEq, Eq)]
struct EditorConfigFile {
    is_root: bool,
    sections: Vec<EditorConfigSection>,
}

#[derive(Debug, PartialEq, Eq)]
struct EditorConfigSection {
    pattern: String,
    properties: Vec<(String, String)>,
}

/// Resolves the effective core properties for `path` by walking its ancestor directories upward,
/// stopping at the first `.editorconfig` that declares `root = true` (or at the filesystem root).
///
/// Precedence is EditorConfig's own: the files are applied outermost-first so the *closest* one
/// wins, and inside a single file the sections are applied top-to-bottom so a later matching
/// section overrides an earlier one. A property whose value is `unset` erases whatever an outer
/// file or earlier section had set.
///
/// `path` is expected to be the root-guard-resolved (canonical) file path the caller is about to
/// open. The walk deliberately ascends past the project root, because that is what EditorConfig
/// means — a repository checked out below a directory that owns the shared config still gets it —
/// and every read it makes is of a file literally named `.editorconfig`, bounded by
/// [`MAX_ANCESTOR_DIRECTORIES`] and [`MAX_EDITORCONFIG_BYTES`].
///
/// Never fails: an unreadable, oversized or malformed file contributes nothing rather than turning
/// opening a file into an error.
pub fn resolve_for_file(path: &Path) -> EditorConfigOptions {
    let Some(start_directory) = path.parent() else {
        return EditorConfigOptions::default();
    };

    let mut chain: Vec<(PathBuf, EditorConfigFile)> = Vec::new();
    for directory in start_directory.ancestors().take(MAX_ANCESTOR_DIRECTORIES) {
        let Some(parsed) = read_editorconfig(&directory.join(EDITORCONFIG_FILE_NAME)) else {
            continue;
        };
        let is_root = parsed.is_root;
        chain.push((directory.to_path_buf(), parsed));
        if is_root {
            break;
        }
    }

    let mut properties: BTreeMap<String, String> = BTreeMap::new();
    for (directory, parsed) in chain.iter().rev() {
        let Some(relative_path) = relative_slash_path(path, directory) else {
            continue;
        };
        for section in &parsed.sections {
            if !section_matches(&section.pattern, &relative_path) {
                continue;
            }
            for (key, value) in &section.properties {
                if value == UNSET_VALUE {
                    properties.remove(key);
                } else {
                    properties.insert(key.clone(), value.clone());
                }
            }
        }
    }

    interpret(&properties)
}

/// Reads and parses one `.editorconfig`, answering `None` for "there is nothing usable here"
/// (absent, not a regular file, larger than [`MAX_EDITORCONFIG_BYTES`], or not valid UTF-8).
fn read_editorconfig(path: &Path) -> Option<EditorConfigFile> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_EDITORCONFIG_BYTES {
        return None;
    }
    Some(parse(&std::fs::read_to_string(path).ok()?))
}

/// Parses the INI-ish `.editorconfig` grammar: `#`/`;` comment lines, `[section]` headers, and
/// `key = value` (or `key: value`) pairs. Keys and values are lowercased — EditorConfig declares
/// both case-insensitive, and every value this module interprets is a keyword, a boolean or a
/// number. `root` is only honored in the preamble (before the first section header), as specified.
///
/// Comments are recognized as whole lines only; a `#` inside a value stays part of the value, which
/// is the conservative reading (an inline-comment rule would silently truncate a glob containing
/// `#`).
fn parse(text: &str) -> EditorConfigFile {
    let mut file = EditorConfigFile::default();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if let Some(pattern) = section_header(line) {
            file.sections.push(EditorConfigSection {
                pattern,
                properties: Vec::new(),
            });
            continue;
        }

        let Some((key, value)) = split_property(line) else {
            continue;
        };

        match file.sections.last_mut() {
            Some(section) => {
                if CORE_PROPERTY_KEYS.contains(&key.as_str()) {
                    section.properties.push((key, value));
                }
            }
            None => {
                if key == ROOT_KEY {
                    file.is_root = value == TRUE_VALUE;
                }
            }
        }
    }

    file
}

fn section_header(line: &str) -> Option<String> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.to_string())
}

fn split_property(line: &str) -> Option<(String, String)> {
    let separator = line
        .find(['=', ':'])
        .filter(|index| *index > 0)
        .filter(|index| !line[..*index].trim().is_empty())?;
    let key = line[..separator].trim().to_lowercase();
    let value = line[separator + 1..].trim().to_lowercase();
    Some((key, value))
}

/// Turns the merged raw property strings into the typed shape the IPC layer carries.
///
/// `indent_size = tab` is resolved here rather than left for the frontend: it means "the width of a
/// tab", which is `tab_width`, and if no `tab_width` was declared there is nothing to resolve it to
/// (the property stays `None` and the frontend falls back to the user's own tab size).
fn interpret(properties: &BTreeMap<String, String>) -> EditorConfigOptions {
    let tab_width = properties.get("tab_width").and_then(|value| parse_indent_columns(value));
    let indent_size = match properties.get("indent_size").map(String::as_str) {
        Some(INDENT_SIZE_TAB_VALUE) => tab_width,
        Some(value) => parse_indent_columns(value),
        None => None,
    };

    EditorConfigOptions {
        indent_style: match properties.get("indent_style").map(String::as_str) {
            Some("tab") => Some(EditorConfigIndentStyle::Tab),
            Some("space") => Some(EditorConfigIndentStyle::Space),
            _ => None,
        },
        indent_size,
        tab_width,
        insert_final_newline: properties.get("insert_final_newline").and_then(|value| parse_bool(value)),
        trim_trailing_whitespace: properties.get("trim_trailing_whitespace").and_then(|value| parse_bool(value)),
    }
}

fn parse_indent_columns(value: &str) -> Option<u32> {
    let columns = value.parse::<u32>().ok()?;
    (1..=MAX_INDENT_COLUMNS).contains(&columns).then_some(columns)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        TRUE_VALUE => Some(true),
        FALSE_VALUE => Some(false),
        _ => None,
    }
}

/// `file`'s path relative to the directory the `.editorconfig` that declared a section lives in,
/// with `/` separators — the string section globs are matched against.
fn relative_slash_path(file: &Path, base: &Path) -> Option<String> {
    let relative = file.strip_prefix(base).ok()?;
    let text = relative.to_string_lossy().replace('\\', "/");
    (!text.is_empty()).then_some(text)
}

/// Whether `pattern` (a section header) selects `relative_path`.
///
/// A pattern with no path separator matches at any depth below the `.editorconfig` (EditorConfig's
/// `*.c` rule), which is expressed by prefixing `**/`; one that has a separator is anchored at the
/// config's own directory, with a single leading `/` stripped.
fn section_matches(pattern: &str, relative_path: &str) -> bool {
    if pattern.is_empty() || pattern.chars().count() > MAX_SECTION_PATTERN_CHARS {
        return false;
    }

    let normalized: Vec<char> = normalize_section_pattern(pattern).chars().collect();
    let nodes = parse_glob(&normalized);
    let node_refs: Vec<&GlobNode> = nodes.iter().collect();
    let mut memo = HashMap::new();
    glob_matches(&node_refs, &relative_path.chars().collect::<Vec<char>>(), &mut memo)
}

fn normalize_section_pattern(pattern: &str) -> String {
    if contains_path_separator(pattern) {
        return pattern.strip_prefix('/').unwrap_or(pattern).to_string();
    }
    format!("**/{pattern}")
}

/// A `/` that is neither escaped nor inside a `[…]` character class — the separator test
/// EditorConfig specifies for deciding whether a section header is anchored.
fn contains_path_separator(pattern: &str) -> bool {
    let mut inside_class = false;
    let mut characters = pattern.chars();

    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                characters.next();
            }
            '[' => inside_class = true,
            ']' => inside_class = false,
            '/' if !inside_class => return true,
            _ => {}
        }
    }

    false
}

#[derive(Debug, PartialEq, Eq)]
enum GlobNode {
    Literal(char),
    /// `?` — one character, never a separator.
    AnyCharacter,
    /// `*` — any run of characters that contains no separator.
    Star,
    /// `**` not followed by `/` — any run of characters, separators included.
    GlobStar,
    /// `**/` — any run of characters ending at a separator, **or nothing at all**, so `**/*.c`
    /// selects `main.c` at the config's own level as well as `src/main.c`. Written as its own node
    /// (rather than `GlobStar` + a literal `/`) precisely to make that zero-directory case work; it
    /// is also what makes the `**/` this module prefixes onto separator-less patterns behave the way
    /// EditorConfig's `*.c` is documented to.
    GlobStarSlash,
    /// `[abc]` / `[!abc]` / `[a-z]` — one character. A negated class never matches a separator.
    CharacterClass {
        negated: bool,
        items: Vec<ClassItem>,
    },
    /// `{a,b,c}` — brace alternation. A brace group without a comma is not this; it stays literal.
    Alternation(Vec<Vec<GlobNode>>),
    /// `{1..9}` — an integer in the inclusive range, written with an optional leading `-`.
    NumericRange(i64, i64),
}

#[derive(Debug, PartialEq, Eq)]
enum ClassItem {
    Single(char),
    Range(char, char),
}

impl ClassItem {
    fn contains(&self, character: char) -> bool {
        match self {
            ClassItem::Single(expected) => *expected == character,
            ClassItem::Range(start, end) => (*start..=*end).contains(&character),
        }
    }
}

fn parse_glob(pattern: &[char]) -> Vec<GlobNode> {
    let mut nodes = Vec::new();
    let mut index = 0;

    while index < pattern.len() {
        match pattern[index] {
            '\\' if index + 1 < pattern.len() => {
                nodes.push(GlobNode::Literal(pattern[index + 1]));
                index += 2;
            }
            '?' => {
                nodes.push(GlobNode::AnyCharacter);
                index += 1;
            }
            '*' => {
                if pattern.get(index + 1) == Some(&'*') {
                    if pattern.get(index + 2) == Some(&'/') {
                        nodes.push(GlobNode::GlobStarSlash);
                        index += 3;
                    } else {
                        nodes.push(GlobNode::GlobStar);
                        index += 2;
                    }
                } else {
                    nodes.push(GlobNode::Star);
                    index += 1;
                }
            }
            '[' => match parse_character_class(pattern, index) {
                Some((node, next)) => {
                    nodes.push(node);
                    index = next;
                }
                None => {
                    nodes.push(GlobNode::Literal('['));
                    index += 1;
                }
            },
            '{' => match parse_brace(pattern, index) {
                Some((node, next)) => {
                    nodes.push(node);
                    index = next;
                }
                None => {
                    nodes.push(GlobNode::Literal('{'));
                    index += 1;
                }
            },
            character => {
                nodes.push(GlobNode::Literal(character));
                index += 1;
            }
        }
    }

    nodes
}

/// Parses `[…]` starting at `start`, answering the node and the index just past the closing `]`.
/// `None` means the class never closes, in which case the caller keeps the `[` as a literal — the
/// same fallback shells use.
fn parse_character_class(pattern: &[char], start: usize) -> Option<(GlobNode, usize)> {
    let mut index = start + 1;
    let negated = pattern.get(index) == Some(&'!');
    if negated {
        index += 1;
    }

    let mut items = Vec::new();
    let mut is_first = true;

    while index < pattern.len() {
        let character = pattern[index];

        if character == ']' && !is_first {
            return Some((GlobNode::CharacterClass { negated, items }, index + 1));
        }
        is_first = false;

        if character == '\\' && index + 1 < pattern.len() {
            items.push(ClassItem::Single(pattern[index + 1]));
            index += 2;
            continue;
        }

        let is_range = pattern.get(index + 1) == Some(&'-') && matches!(pattern.get(index + 2), Some(end) if *end != ']');
        if is_range {
            items.push(ClassItem::Range(character, pattern[index + 2]));
            index += 3;
            continue;
        }

        items.push(ClassItem::Single(character));
        index += 1;
    }

    None
}

/// Parses `{…}` starting at `start`. `None` means "not an alternation" — either the brace never
/// closes, or it holds no top-level comma and is not a `{n..m}` range, which EditorConfig specifies
/// as literal text (`{word}` matches the six characters, not `word`).
fn parse_brace(pattern: &[char], start: usize) -> Option<(GlobNode, usize)> {
    let end = matching_brace(pattern, start)?;
    let inner = &pattern[start + 1..end];

    if let Some(range) = parse_numeric_range(inner) {
        return Some((range, end + 1));
    }

    let alternatives = split_top_level_commas(inner);
    if alternatives.len() < 2 {
        return None;
    }

    Some((GlobNode::Alternation(alternatives.into_iter().map(parse_glob).collect()), end + 1))
}

fn matching_brace(pattern: &[char], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = start;

    while index < pattern.len() {
        match pattern[index] {
            '\\' => index += 1,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn parse_numeric_range(inner: &[char]) -> Option<GlobNode> {
    let text: String = inner.iter().collect();
    let (start, end) = text.split_once("..")?;
    Some(GlobNode::NumericRange(start.parse().ok()?, end.parse().ok()?))
}

fn split_top_level_commas(inner: &[char]) -> Vec<&[char]> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut part_start = 0usize;
    let mut index = 0usize;

    while index < inner.len() {
        match inner[index] {
            '\\' => index += 1,
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&inner[part_start..index]);
                part_start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }

    parts.push(&inner[part_start.min(inner.len())..]);
    parts
}

/// Matches `nodes` against the whole of `input` (an anchored match — the pattern must consume the
/// entire relative path), backtracking through the wildcards.
///
/// Takes `&[&GlobNode]` rather than `&[GlobNode]` so an [`GlobNode::Alternation`] branch can be
/// spliced in front of the remaining nodes without cloning the tree.
///
/// `memo` is what keeps the backtracking polynomial instead of exponential: without it a pattern
/// whose wildcards overlap (`*a*a*a…` against a long run of `a`s) re-explores the same
/// (node, position) pair once per way of reaching it, which a hostile `.editorconfig` in an
/// untrusted repository could turn into a file open that never returns. The key is sound because
/// each node lives at exactly one place in the parsed tree, and the continuation after a node is
/// therefore fixed: the nodes still to match after it are the rest of its own list followed by
/// whatever followed the alternation that list belongs to, all the way up. So the head node's
/// address plus how much input is left names the state completely, spliced alternation branches
/// included.
fn glob_matches(nodes: &[&GlobNode], input: &[char], memo: &mut HashMap<(usize, usize), bool>) -> bool {
    let Some((node, rest)) = nodes.split_first() else {
        return input.is_empty();
    };

    let state = (std::ptr::from_ref(*node) as usize, input.len());
    if let Some(matched) = memo.get(&state) {
        return *matched;
    }

    let matched = match node {
        GlobNode::Literal(expected) => match input.split_first() {
            Some((actual, tail)) if actual == expected => glob_matches(rest, tail, memo),
            _ => false,
        },
        GlobNode::AnyCharacter => match input.split_first() {
            Some((actual, tail)) if *actual != '/' => glob_matches(rest, tail, memo),
            _ => false,
        },
        GlobNode::Star => {
            let mut star_matched = false;
            for consumed in 0..=input.len() {
                if consumed > 0 && input[consumed - 1] == '/' {
                    break;
                }
                if glob_matches(rest, &input[consumed..], memo) {
                    star_matched = true;
                    break;
                }
            }
            star_matched
        }
        GlobNode::GlobStar => (0..=input.len()).any(|consumed| glob_matches(rest, &input[consumed..], memo)),
        GlobNode::GlobStarSlash => {
            glob_matches(rest, input, memo)
                || (1..=input.len()).any(|consumed| input[consumed - 1] == '/' && glob_matches(rest, &input[consumed..], memo))
        }
        GlobNode::CharacterClass { negated, items } => match input.split_first() {
            Some((actual, tail)) => {
                let listed = items.iter().any(|item| item.contains(*actual));
                let selected = if *negated { !listed && *actual != '/' } else { listed };
                selected && glob_matches(rest, tail, memo)
            }
            None => false,
        },
        GlobNode::Alternation(alternatives) => alternatives.iter().any(|alternative| {
            let mut spliced: Vec<&GlobNode> = alternative.iter().collect();
            spliced.extend(rest.iter().copied());
            glob_matches(&spliced, input, memo)
        }),
        GlobNode::NumericRange(minimum, maximum) => {
            let sign_length = usize::from(input.first() == Some(&'-'));
            let mut digit_end = sign_length;
            while digit_end < input.len() && input[digit_end].is_ascii_digit() {
                digit_end += 1;
            }

            (sign_length + 1..=digit_end).rev().any(|length| {
                let text: String = input[..length].iter().collect();
                text.parse::<i64>().is_ok_and(|value| (*minimum..=*maximum).contains(&value)) && glob_matches(rest, &input[length..], memo)
            })
        }
    };

    memo.insert(state, matched);
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("taide-editorconfig-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("임시 디렉토리 생성");
        directory
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("부모 디렉토리 생성");
        }
        std::fs::write(path, content).expect("파일 쓰기");
    }

    #[test]
    fn 섹션과_프로퍼티를_파싱하고_코어_키만_남긴다() {
        let parsed = parse(
            r#"
# comment
root = true
; another comment

[*.rs]
indent_style = tab
indent_size = 4
charset = utf-8

[*.md]
trim_trailing_whitespace = false
"#,
        );

        assert!(parsed.is_root);
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].pattern, "*.rs");
        assert_eq!(
            parsed.sections[0].properties,
            vec![
                ("indent_style".to_string(), "tab".to_string()),
                ("indent_size".to_string(), "4".to_string())
            ]
        );
        assert_eq!(
            parsed.sections[1].properties,
            vec![("trim_trailing_whitespace".to_string(), "false".to_string())]
        );
    }

    #[test]
    fn 키와_값은_대소문자를_구분하지_않고_콜론_구분자도_받는다() {
        let parsed = parse("[*]\nIndent_Style : SPACE\nINDENT_SIZE = 2\n");

        assert_eq!(
            parsed.sections[0].properties,
            vec![
                ("indent_style".to_string(), "space".to_string()),
                ("indent_size".to_string(), "2".to_string())
            ]
        );
    }

    #[test]
    fn 섹션_안의_root_는_무시된다() {
        let parsed = parse("[*]\nroot = true\n");

        assert!(!parsed.is_root);
    }

    #[test]
    fn 별표는_구분자를_넘지_않고_이중별표는_넘는다() {
        assert!(section_matches("*.c", "main.c"));
        assert!(section_matches("*.c", "src/deep/main.c"));
        assert!(section_matches("src/*.c", "src/main.c"));
        assert!(!section_matches("src/*.c", "src/deep/main.c"));
        assert!(section_matches("src/**.c", "src/deep/main.c"));
        assert!(section_matches("lib/**", "lib/a/b/c.rs"));
    }

    #[test]
    fn 이중별표_슬래시는_디렉토리_0개도_받는다() {
        assert!(section_matches("**/main.c", "main.c"));
        assert!(section_matches("**/main.c", "a/b/main.c"));
        assert!(section_matches("src/**/main.c", "src/main.c"));
        assert!(section_matches("src/**/main.c", "src/a/main.c"));
    }

    #[test]
    fn 선행_슬래시는_설정_파일_디렉토리에_고정한다() {
        assert!(section_matches("/main.c", "main.c"));
        assert!(!section_matches("/main.c", "src/main.c"));
    }

    #[test]
    fn 물음표와_문자클래스를_매칭한다() {
        assert!(section_matches("?.c", "a.c"));
        assert!(!section_matches("?.c", "ab.c"));
        assert!(section_matches("[abc].c", "b.c"));
        assert!(section_matches("[a-z].c", "q.c"));
        assert!(!section_matches("[a-z].c", "Q.c"));
        assert!(section_matches("[!a].c", "b.c"));
        assert!(!section_matches("[!a].c", "a.c"));
    }

    #[test]
    fn 중괄호_교대는_콤마가_있을_때만_적용되고_없으면_리터럴이다() {
        assert!(section_matches("*.{js,ts}", "main.ts"));
        assert!(section_matches("*.{js,ts}", "main.js"));
        assert!(!section_matches("*.{js,ts}", "main.rs"));
        assert!(section_matches("{word}.c", "{word}.c"));
        assert!(!section_matches("{word}.c", "word.c"));
    }

    #[test]
    fn 중괄호_숫자범위를_매칭한다() {
        assert!(section_matches("file{1..3}.c", "file2.c"));
        assert!(!section_matches("file{1..3}.c", "file4.c"));
        assert!(section_matches("file{-2..2}.c", "file-1.c"));
        assert!(section_matches("page{1..100}.md", "page42.md"));
    }

    #[test]
    fn 역슬래시_이스케이프는_와일드카드를_리터럴로_만든다() {
        assert!(section_matches("a\\*b.c", "a*b.c"));
        assert!(!section_matches("a\\*b.c", "axb.c"));
    }

    #[test]
    fn 와일드카드가_겹쳐도_지수적으로_퍼지지_않는다() {
        let overlapping_stars = "*a".repeat(24);
        let long_run = "a".repeat(64);

        assert!(!section_matches(&format!("{overlapping_stars}z"), &long_run));
        assert!(section_matches(&overlapping_stars, &long_run));
    }

    #[test]
    fn 가까운_설정과_뒤에_오는_섹션이_이긴다() {
        let root = temp_dir("precedence");
        write(
            &root.join(".editorconfig"),
            "root = true\n[*]\nindent_style = space\nindent_size = 4\n[*.rs]\nindent_size = 8\n",
        );
        write(&root.join("src/.editorconfig"), "[*.rs]\nindent_size = 2\n");
        write(&root.join("src/main.rs"), "");

        let options = resolve_for_file(&root.join("src/main.rs"));

        assert_eq!(options.indent_style, Some(EditorConfigIndentStyle::Space));
        assert_eq!(options.indent_size, Some(2));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn root_true_위쪽_설정은_읽지_않는다() {
        let outer = temp_dir("root-stop");
        write(&outer.join(".editorconfig"), "[*]\ninsert_final_newline = true\n");
        write(&outer.join("inner/.editorconfig"), "root = true\n[*]\nindent_size = 3\n");
        write(&outer.join("inner/main.rs"), "");

        let options = resolve_for_file(&outer.join("inner/main.rs"));

        assert_eq!(options.indent_size, Some(3));
        assert_eq!(options.insert_final_newline, None);

        let _ = std::fs::remove_dir_all(&outer);
    }

    #[test]
    fn unset_은_바깥_설정을_지운다() {
        let root = temp_dir("unset");
        write(
            &root.join(".editorconfig"),
            "root = true\n[*]\ntrim_trailing_whitespace = true\n[*.md]\ntrim_trailing_whitespace = unset\n",
        );
        write(&root.join("notes.md"), "");
        write(&root.join("main.rs"), "");

        assert_eq!(resolve_for_file(&root.join("notes.md")).trim_trailing_whitespace, None);
        assert_eq!(resolve_for_file(&root.join("main.rs")).trim_trailing_whitespace, Some(true));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn indent_size_tab_은_tab_width_로_해석된다() {
        let root = temp_dir("indent-size-tab");
        write(
            &root.join(".editorconfig"),
            "root = true\n[*]\nindent_style = tab\nindent_size = tab\ntab_width = 8\n",
        );
        write(&root.join("main.go"), "");

        let options = resolve_for_file(&root.join("main.go"));

        assert_eq!(options.indent_style, Some(EditorConfigIndentStyle::Tab));
        assert_eq!(options.tab_width, Some(8));
        assert_eq!(options.indent_size, Some(8));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn 범위를_벗어나거나_형식이_틀린_값은_무시된다() {
        let root = temp_dir("invalid-values");
        write(
            &root.join(".editorconfig"),
            "root = true\n[*]\nindent_size = 0\ntab_width = 999\ninsert_final_newline = yes\nindent_style = spaces\n",
        );
        write(&root.join("main.rs"), "");

        let options = resolve_for_file(&root.join("main.rs"));

        assert_eq!(options.indent_size, None);
        assert_eq!(options.tab_width, None);
        assert_eq!(options.insert_final_newline, None);
        assert_eq!(options.indent_style, None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn 어떤_섹션도_맞지_않으면_전부_비어_있다() {
        let root = temp_dir("no-match");
        write(&root.join(".editorconfig"), "root = true\n[*.rs]\nindent_size = 2\n");
        write(&root.join("notes.md"), "");

        assert_eq!(resolve_for_file(&root.join("notes.md")), EditorConfigOptions::default());

        let _ = std::fs::remove_dir_all(&root);
    }
}
