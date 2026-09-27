use std::path::PathBuf;

use taide_model::error::AppErrorKind;
use taide_model::paths::AppPaths;
use taide_runtime::{snippet_actions, AppState};
use uuid::Uuid;

struct Fixture(AppState);

impl Fixture {
    fn new() -> Self {
        Self(AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-snippet-actions-{}", Uuid::new_v4())),
        )))
    }

    fn path(&self, file_name: &str) -> PathBuf {
        self.0.paths.snippets_dir().join(file_name)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.paths.data_dir.exists() {
            std::fs::remove_dir_all(&self.0.paths.data_dir).expect("자기 UUID snippet fixture만 정리");
        }
    }
}

#[test]
fn 없는_목록과_삭제는_기존_오류이고_디렉터리를_생성하지_않는다() {
    let fixture = Fixture::new();
    assert!(snippet_actions::snippet_list(&fixture.0).unwrap().is_empty());
    let error = snippet_actions::snippet_delete(&fixture.0, "missing.json".to_string()).unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(!fixture.0.paths.data_dir.exists());
}

#[test]
fn 저장_원문과_정렬_관용_목록_삭제를_같은_service로_유지한다() {
    let fixture = Fixture::new();
    let content = "{\n  \"Fixture\": { \"prefix\": [\"a\", \"b\"], \"body\": [\"line\"], \"ignored\": true }\n}\n";
    let saved = snippet_actions::snippet_save(&fixture.0, "z.code-snippets".to_string(), content.to_string()).unwrap();
    assert_eq!(saved.file_name, "z.code-snippets");
    assert!(saved.snippets.contains_key("Fixture"));
    assert_eq!(std::fs::read_to_string(fixture.path("z.code-snippets")).unwrap(), content);
    snippet_actions::snippet_save(&fixture.0, "a.json".to_string(), "{}".to_string()).unwrap();
    std::fs::write(fixture.path("broken.json"), b"{broken").unwrap();
    std::fs::write(fixture.path("ignored.txt"), b"{}").unwrap();
    let files = snippet_actions::snippet_list(&fixture.0).unwrap();
    assert_eq!(
        files.iter().map(|file| file.file_name.as_str()).collect::<Vec<_>>(),
        ["a.json", "z.code-snippets"]
    );
    snippet_actions::snippet_delete(&fixture.0, "a.json".to_string()).unwrap();
    assert!(!fixture.path("a.json").exists());
    assert_eq!(snippet_actions::snippet_list(&fixture.0).unwrap().len(), 1);
    assert!(fixture.path("broken.json").exists());
    assert!(fixture.path("ignored.txt").exists());
}

#[test]
fn 경로와_json_검증_실패는_기존_파일을_바꾸지_않는다() {
    let fixture = Fixture::new();
    for file_name in [
        "",
        "../escape.json",
        "sub/escape.json",
        "sub\\escape.json",
        "C:escape.json",
        "notes.txt",
    ] {
        let error = snippet_actions::snippet_save(&fixture.0, file_name.to_string(), "{}".to_string()).unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        let error = snippet_actions::snippet_delete(&fixture.0, file_name.to_string()).unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    }
    assert!(!fixture.0.paths.data_dir.exists());
    snippet_actions::snippet_save(&fixture.0, "fixture.json".to_string(), "{}".to_string()).unwrap();
    let error = snippet_actions::snippet_save(&fixture.0, "fixture.json".to_string(), "{broken".to_string()).unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert_eq!(std::fs::read_to_string(fixture.path("fixture.json")).unwrap(), "{}");
}

#[test]
fn 공개_세_action은_runtime에_위임한다() {
    let source = include_str!("../src/domain/snippet/commands.rs");
    for name in ["snippet_list", "snippet_save", "snippet_delete"] {
        let body = source
            .split_once(&format!("pub async fn {name}("))
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(body.contains(&format!("snippet_actions::{name}(")), "{name}");
    }
}
