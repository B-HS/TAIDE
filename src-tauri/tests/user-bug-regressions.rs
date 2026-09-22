use std::collections::HashMap;
use std::path::{Path, PathBuf};

use taide_lib::domain::{file, git, project, settings, sync, task};
use taide_lib::ids::ProjectId;
use taide_lib::infra::root_guard;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("taide-user-bug-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        Self {
            root: root.canonicalize().unwrap(),
        }
    }

    fn projects(&self) -> HashMap<ProjectId, project::types::Project> {
        let id = ProjectId::from("audit".to_string());
        HashMap::from([(
            id.clone(),
            project::types::Project {
                id,
                root: self.root.to_string_lossy().into_owned(),
                name: "audit".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        )])
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[cfg(unix)]
#[test]
fn 링크_개명과_삭제는_원본을_보존한다() {
    let fixture = Fixture::new();
    let target = fixture.root.join("target.txt");
    let link = fixture.root.join("link.txt");
    let renamed = fixture.root.join("renamed.txt");
    std::fs::write(&target, "original").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let projects = fixture.projects();
    let (_, from) = root_guard::resolve_entry_owning_project(&projects, &link).unwrap();
    let (_, to) = root_guard::resolve_entry_owning_project(&projects, &renamed).unwrap();
    file::service::rename_entry(&from, &to).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
    assert!(renamed.symlink_metadata().unwrap().is_symlink());
    assert!(!link.exists());

    let (_, resolved) = root_guard::resolve_entry_owning_project(&projects, &renamed).unwrap();
    file::service::delete_entry(&resolved).unwrap();
    assert!(renamed.symlink_metadata().is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
}

#[cfg(unix)]
#[test]
fn 외부를_가리키는_링크_자체는_개명하지만_외부_디렉터리_내부는_거부한다() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let link = fixture.root.join("outside");
    std::os::unix::fs::symlink(&outside.root, &link).unwrap();
    let projects = fixture.projects();
    assert!(root_guard::resolve_entry_owning_project(&projects, &link.join("file.txt")).is_err());
    let (_, resolved) = root_guard::resolve_entry_owning_project(&projects, &link).unwrap();
    file::service::rename_entry(&resolved, &fixture.root.join("moved-link")).unwrap();
    assert!(outside.root.is_dir());
    assert!(fixture.root.join("moved-link").symlink_metadata().unwrap().is_symlink());
    assert!(root_guard::resolve_entry_owning_project(&projects, &fixture.root).is_err());
}

#[cfg(unix)]
#[test]
fn 같은_원본의_별도_링크나_원본을_덮어쓰지_않는다() {
    let fixture = Fixture::new();
    let target = fixture.root.join("target");
    let source = fixture.root.join("source");
    let other = fixture.root.join("other");
    std::fs::write(&target, "original").unwrap();
    std::os::unix::fs::symlink(&target, &source).unwrap();
    std::os::unix::fs::symlink(&target, &other).unwrap();
    assert!(file::service::rename_entry(&source, &target).is_err());
    assert!(file::service::rename_entry(&source, &other).is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
    assert!(source.symlink_metadata().unwrap().is_symlink());
    assert!(other.symlink_metadata().unwrap().is_symlink());
}

#[cfg(unix)]
#[test]
fn 링크의_대소문자_개명도_요청한_표기를_유지한다() {
    let fixture = Fixture::new();
    let target = fixture.root.join("target");
    let source = fixture.root.join("link.txt");
    let destination = fixture.root.join("LINK.txt");
    std::fs::write(&target, "original").unwrap();
    std::os::unix::fs::symlink(&target, &source).unwrap();
    let projects = fixture.projects();
    let (_, from) = root_guard::resolve_entry_owning_project(&projects, &source).unwrap();
    let (_, to) = root_guard::resolve_entry_owning_project(&projects, &destination).unwrap();
    file::service::rename_entry(&from, &to).unwrap();
    let names: Vec<_> = std::fs::read_dir(&fixture.root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(names.iter().any(|name| name == "LINK.txt"));
    assert!(!names.iter().any(|name| name == "link.txt"));
    assert!(destination.symlink_metadata().unwrap().is_symlink());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
}

#[cfg(unix)]
#[test]
fn 깨진_링크와_디렉터리_링크는_링크로_스테이징된다() {
    const SYMLINK_MODE: u32 = 0o120000;
    let fixture = Fixture::new();
    let repo = git2::Repository::init(&fixture.root).unwrap();
    std::os::unix::fs::symlink("missing.txt", fixture.root.join("broken")).unwrap();
    std::fs::create_dir(fixture.root.join("directory")).unwrap();
    std::fs::write(fixture.root.join("directory/file.txt"), "content").unwrap();
    std::os::unix::fs::symlink("directory", fixture.root.join("directory-link")).unwrap();
    git::service::stage(&fixture.root, &["broken".into(), "directory-link".into()]).unwrap();
    let index = repo.index().unwrap();
    for name in ["broken", "directory-link"] {
        assert_eq!(index.get_path(Path::new(name), 0).unwrap().mode, SYMLINK_MODE);
    }
    assert!(index.get_path(Path::new("directory-link/file.txt"), 0).is_none());
    let diff = git::service::diff_file(&fixture.root, "broken", git::types::DiffMode::WorkdirVsIndex, None, &[]).unwrap();
    assert_eq!(diff.original, "missing.txt");
    assert_eq!(diff.modified, "missing.txt");
}

#[test]
fn 읽을_수_없는_비어있지_않은_파일은_삭제로_표시하지_않는다() {
    const INVALID_UTF8: &[u8] = &[0xff, b'\n'];
    let fixture = Fixture::new();
    let repo = git2::Repository::init(&fixture.root).unwrap();
    let path = fixture.root.join("source.txt");
    std::fs::write(&path, "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("source.txt")).unwrap();
    index.write().unwrap();
    std::fs::write(&path, INVALID_UTF8).unwrap();
    assert!(git::service::diff_file(&fixture.root, "source.txt", git::types::DiffMode::WorkdirVsIndex, None, &[]).is_err());
    std::fs::remove_file(&path).unwrap();
    let diff = git::service::diff_file(&fixture.root, "source.txt", git::types::DiffMode::WorkdirVsIndex, None, &[]).unwrap();
    assert_eq!(diff.original, "hello\n");
    assert_eq!(diff.modified, "");
}

#[test]
fn 동기화는_글꼴과_모델의_기본값_복원을_전달한다() {
    let local = settings::types::Settings {
        editor_font_family: Some("Fira Code".into()),
        terminal_font_family: Some("Fira Code".into()),
        ui_font_family: Some("Custom".into()),
        ai_model: Some("custom-model".into()),
        ..Default::default()
    };
    let payload = sync::service::assemble_payload(&Default::default(), Vec::new(), Vec::new(), "2026-09-22T00:00:00Z".into());
    let applied = sync::service::apply_payload_settings(&local, &payload);
    assert!(applied.editor_font_family.is_none());
    assert!(applied.terminal_font_family.is_none());
    assert!(applied.ui_font_family.is_none());
    assert!(applied.ai_model.is_none());

    let mut legacy = serde_json::to_value(payload).unwrap();
    legacy["settings"]["editorFontFamily"] = serde_json::Value::Null;
    legacy["settings"].as_object_mut().unwrap().remove("terminalFontFamily");
    let parsed = sync::service::parse_synced_payload(&legacy.to_string()).unwrap();
    let applied = sync::service::apply_payload_settings(&local, &parsed);
    assert!(applied.editor_font_family.is_none());
    assert_eq!(applied.terminal_font_family, local.terminal_font_family);
}

#[test]
fn 여러_make_타깃과_공백_이스케이프를_독립_명령으로_실행한다() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("Makefile"), "build test file\\ name:\n\t@echo done\n").unwrap();
    let tasks = task::service::detect_tasks(&fixture.root);
    let labels: Vec<_> = tasks.iter().map(|task| task.label.as_str()).collect();
    assert_eq!(labels, ["build", "test", "file name"]);
    for task in tasks {
        let output = std::process::Command::new("sh")
            .args(["-c", &task.command])
            .current_dir(&fixture.root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}: {}",
            task.command,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
