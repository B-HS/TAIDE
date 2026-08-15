use crate::domain::snippet::types::{SnippetFile, SnippetMap};
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::paths::AppPaths;

const SNIPPET_LANGUAGE_EXTENSION: &str = "json";
const SNIPPET_GLOBAL_EXTENSION: &str = "code-snippets";

fn snippet_extension(file_name: &str) -> Option<&str> {
    std::path::Path::new(file_name).extension().and_then(|value| value.to_str())
}

/// A snippet file is either a per-language `<languageId>.json` or a global
/// `*.code-snippets` file (VS Code's own naming split — see the contract
/// §3.3). Both [`list_snippet_files`]'s directory scan and
/// [`sanitize_snippet_file_name`]'s save-time check share this predicate so
/// the two can never drift apart: a name `save_snippet_file` accepts is
/// always a name `list_snippet_files` will later surface, so a saved file
/// never goes silently invisible to the frontend's completion provider.
fn has_recognized_snippet_extension(file_name: &str) -> bool {
    matches!(
        snippet_extension(file_name),
        Some(SNIPPET_LANGUAGE_EXTENSION) | Some(SNIPPET_GLOBAL_EXTENSION)
    )
}

/// Rejects `/`, `\`, `..`, and `:`. The first three keep `file_name` inside `snippets_dir()` on
/// their own; `:` closes a separate escape those three don't: on Windows, a name like
/// `"C:evil.json"` carries none of them, yet `PathBuf::push`/`join` still discards the
/// `snippets_dir()` base and resolves the join relative to drive `C`'s own current directory
/// (`PathBuf::push`'s documented behavior — "if `path` has a prefix but no root, it replaces
/// `self`"), escaping the snippets directory entirely for both save and delete. The check is a
/// plain substring match rather than `std::path::Component` parsing on purpose: `Component`'s
/// Windows-prefix recognition is target-conditional, so the same check written against it would
/// silently stop catching this on a non-Windows build/test host even though the vulnerability is
/// still live in a Windows build. Rejecting `:` outright also closes the unrelated NTFS
/// alternate-data-stream form (`"evil.json:hidden"`) as a side effect.
fn has_unsafe_path_characters(file_name: &str) -> bool {
    file_name.contains(['/', '\\', ':']) || file_name.contains("..")
}

fn sanitize_snippet_file_name(file_name: &str) -> AppResult<()> {
    if file_name.trim().is_empty() {
        return Err(AppError::InvalidArgument("snippet file name must not be empty".to_string()));
    }
    if has_unsafe_path_characters(file_name) {
        return Err(AppError::InvalidArgument(format!("invalid snippet file name: {file_name}")));
    }
    if !has_recognized_snippet_extension(file_name) {
        return Err(AppError::InvalidArgument(format!(
            "snippet file name must end with .{SNIPPET_LANGUAGE_EXTENSION} or .{SNIPPET_GLOBAL_EXTENSION}: {file_name}"
        )));
    }
    Ok(())
}

/// Scans `snippets_dir()` for `<languageId>.json`/`*.code-snippets` files.
/// Follows `theme::service::list_themes`'s tolerant-scan precedent — a
/// malformed file never fails the whole listing — but additionally logs the
/// skip, per the contract §3.3 (`theme_list` stays silent on skip; this
/// domain does not).
pub fn list_snippet_files(paths: &AppPaths) -> Vec<SnippetFile> {
    let mut list = Vec::new();

    let Ok(entries) = std::fs::read_dir(paths.snippets_dir()) else {
        return list;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !has_recognized_snippet_extension(file_name) {
            continue;
        }

        match persist::read_json::<SnippetMap>(&path) {
            Ok(Some(snippets)) => list.push(SnippetFile {
                file_name: file_name.to_string(),
                snippets,
            }),
            Ok(None) => {}
            Err(error) => log::warn!("스니펫 파일 파싱 실패, 건너뜀: {file_name} ({error})"),
        }
    }

    list.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    list
}

/// Validates `content` as JSON matching [`SnippetMap`] before writing it
/// verbatim (not re-serialized) to `snippets_dir()/{file_name}`, so the
/// caller's own formatting/key order survives the round trip.
pub fn save_snippet_file(paths: &AppPaths, file_name: &str, content: &str) -> AppResult<SnippetFile> {
    sanitize_snippet_file_name(file_name)?;
    let snippets: SnippetMap =
        serde_json::from_str(content).map_err(|error| AppError::InvalidArgument(format!("invalid snippet json: {error}")))?;

    std::fs::create_dir_all(paths.snippets_dir())?;
    persist::write_atomic(&paths.snippets_dir().join(file_name), content.as_bytes())?;

    Ok(SnippetFile {
        file_name: file_name.to_string(),
        snippets,
    })
}

pub fn delete_snippet_file(paths: &AppPaths, file_name: &str) -> AppResult<()> {
    sanitize_snippet_file_name(file_name)?;
    let path = paths.snippets_dir().join(file_name);
    if !path.exists() {
        return Err(AppError::NotFound(format!("snippet file not found: {file_name}")));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-snippet-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn cleanup(paths: &AppPaths) {
        std::fs::remove_dir_all(&paths.data_dir).ok();
    }

    #[test]
    fn 스니펫_디렉토리가_없으면_빈_목록을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("missing-dir"));
        assert!(list_snippet_files(&paths).is_empty());
    }

    #[test]
    fn 유효한_스니펫_파일을_저장하고_목록에서_읽는다() {
        let paths = AppPaths::new(temp_data_dir("save-list"));
        let content = r#"{
            "For Loop": {
                "prefix": "for",
                "body": ["for (let i = 0; i < ${1:n}; i++) {", "\t$0", "}"],
                "description": "A basic for loop"
            }
        }"#;

        let saved = save_snippet_file(&paths, "javascript.json", content).expect("save");
        assert_eq!(saved.file_name, "javascript.json");
        assert!(saved.snippets.contains_key("For Loop"));

        let listed = list_snippet_files(&paths);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].file_name, "javascript.json");

        cleanup(&paths);
    }

    #[test]
    fn prefix가_단일_문자열이어도_배열이어도_파싱된다() {
        let paths = AppPaths::new(temp_data_dir("prefix-shapes"));
        let content = r#"{
            "Single": { "prefix": "single", "body": "single body" },
            "Multi": { "prefix": ["a", "b"], "body": ["line1", "line2"], "scope": "typescript,typescriptreact" }
        }"#;

        let saved = save_snippet_file(&paths, "global.code-snippets", content).expect("save");
        assert_eq!(saved.snippets.len(), 2);
        assert_eq!(
            saved.snippets.get("Multi").and_then(|entry| entry.scope.as_deref()),
            Some("typescript,typescriptreact")
        );

        cleanup(&paths);
    }

    #[test]
    fn 스키마에_없는_필드는_무시하고_관용적으로_파싱한다() {
        let paths = AppPaths::new(temp_data_dir("lenient"));
        let content = r#"{
            "Legacy": {
                "prefix": "legacy",
                "body": "legacy body",
                "isFileTemplate": true,
                "include": ["*.ts"]
            }
        }"#;

        let saved = save_snippet_file(&paths, "legacy.json", content).expect("save");
        assert!(saved.snippets.contains_key("Legacy"));

        cleanup(&paths);
    }

    #[test]
    fn 파싱_실패_파일은_건너뛰고_나머지는_목록에_남는다() {
        let paths = AppPaths::new(temp_data_dir("skip-broken"));
        std::fs::create_dir_all(paths.snippets_dir()).expect("create snippets dir");
        std::fs::write(paths.snippets_dir().join("broken.json"), b"{not json").expect("write broken file");
        save_snippet_file(&paths, "ok.json", r#"{"Ok": {"prefix": "ok", "body": "ok"}}"#).expect("save valid file");

        let listed = list_snippet_files(&paths);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].file_name, "ok.json");

        cleanup(&paths);
    }

    #[test]
    fn json도_code_snippets도_아닌_확장자는_목록에서_제외된다() {
        let paths = AppPaths::new(temp_data_dir("wrong-ext"));
        std::fs::create_dir_all(paths.snippets_dir()).expect("create snippets dir");
        std::fs::write(paths.snippets_dir().join("notes.txt"), b"{}").expect("write stray file");

        assert!(list_snippet_files(&paths).is_empty());

        cleanup(&paths);
    }

    #[test]
    fn 잘못된_json은_저장을_거부한다() {
        let paths = AppPaths::new(temp_data_dir("invalid-json"));
        let result = save_snippet_file(&paths, "broken.json", "{not json");
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn 경로_구분자가_섞인_파일명은_거부한다() {
        let paths = AppPaths::new(temp_data_dir("path-sep"));
        let content = r#"{"A": {"prefix": "a", "body": "a"}}"#;

        assert!(matches!(
            save_snippet_file(&paths, "../evil.json", content),
            Err(AppError::InvalidArgument(_))
        ));
        assert!(matches!(
            save_snippet_file(&paths, "sub/evil.json", content),
            Err(AppError::InvalidArgument(_))
        ));
        assert!(matches!(
            save_snippet_file(&paths, "sub\\evil.json", content),
            Err(AppError::InvalidArgument(_))
        ));
    }

    #[test]
    fn windows_드라이브_상대경로_파일명은_거부한다() {
        let paths = AppPaths::new(temp_data_dir("drive-relative"));
        let content = r#"{"A": {"prefix": "a", "body": "a"}}"#;

        assert!(matches!(
            save_snippet_file(&paths, "C:evil.json", content),
            Err(AppError::InvalidArgument(_))
        ));
        assert!(matches!(
            delete_snippet_file(&paths, "C:evil.json"),
            Err(AppError::InvalidArgument(_))
        ));
    }

    #[test]
    fn 인식되지_않는_확장자는_저장을_거부한다() {
        let paths = AppPaths::new(temp_data_dir("wrong-ext-save"));
        let content = r#"{"A": {"prefix": "a", "body": "a"}}"#;

        assert!(matches!(
            save_snippet_file(&paths, "notes.txt", content),
            Err(AppError::InvalidArgument(_))
        ));
    }

    #[test]
    fn 저장된_파일을_삭제한다() {
        let paths = AppPaths::new(temp_data_dir("delete"));
        save_snippet_file(&paths, "temp.json", r#"{"A": {"prefix": "a", "body": "a"}}"#).expect("save");

        delete_snippet_file(&paths, "temp.json").expect("delete");
        assert!(list_snippet_files(&paths).is_empty());

        cleanup(&paths);
    }

    #[test]
    fn 없는_파일_삭제는_notfound를_반환한다() {
        let paths = AppPaths::new(temp_data_dir("delete-missing"));
        let result = delete_snippet_file(&paths, "does-not-exist.json");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
