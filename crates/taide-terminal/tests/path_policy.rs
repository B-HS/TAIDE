use std::collections::HashMap;
use std::path::{Path, PathBuf};

use taide_model::error::AppError;
use taide_model::ids::ProjectId;
use taide_model::project::Project;
use taide_terminal::service::{guard_terminal_path, resolve_link_candidates, MAX_LINK_CANDIDATES_PER_ROW};

fn single_project(root: &Path) -> HashMap<ProjectId, Project> {
    let id = ProjectId::from("project-1".to_string());
    HashMap::from([(
        id.clone(),
        Project {
            id,
            root: root.to_string_lossy().to_string(),
            name: "project".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    )])
}

fn fixture_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("taide-terminal-path-policy-{name}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("project/src")).expect("프로젝트 생성");
    std::fs::write(root.join("project/src/main.rs"), b"source").expect("프로젝트 파일 생성");
    root
}

#[test]
fn 프로젝트_안의_링크만_정규_경로로_반환한다() {
    let fixture = fixture_root("inside");
    let root = fixture.join("project");
    let projects = single_project(&root);

    let resolved = guard_terminal_path(&projects, "src/main.rs", &root.to_string_lossy()).expect("프로젝트 안 파일");

    assert_eq!(resolved, root.join("src/main.rs").canonicalize().unwrap().to_string_lossy());
    std::fs::remove_dir_all(fixture).expect("fixture 정리");
}

#[test]
fn 루트_밖과_없는_링크는_모두_notfound로_거부한다() {
    let fixture = fixture_root("outside");
    let root = fixture.join("project");
    let outside = fixture.join("outside");
    std::fs::create_dir_all(&outside).expect("외부 디렉터리 생성");
    std::fs::write(outside.join("secret.txt"), b"secret").expect("외부 파일 생성");
    let projects = single_project(&root);

    let outside_error = guard_terminal_path(&projects, "secret.txt", &outside.to_string_lossy()).expect_err("루트 밖 거부");
    let missing_error = guard_terminal_path(&projects, "missing.txt", &root.to_string_lossy()).expect_err("없는 파일 거부");

    assert!(matches!(outside_error, AppError::NotFound(_)));
    assert!(matches!(missing_error, AppError::NotFound(_)));
    std::fs::remove_dir_all(fixture).expect("fixture 정리");
}

#[test]
fn 후보_상한과_입력_순서를_보존하고_거부는_none으로_접는다() {
    let fixture = fixture_root("candidates");
    let root = fixture.join("project");
    let projects = single_project(&root);
    let outside_dir = fixture.join("outside");
    std::fs::create_dir_all(&outside_dir).expect("외부 디렉터리 생성");
    std::fs::write(outside_dir.join("secret.txt"), b"secret").expect("외부 파일 생성");
    let names = (0..=MAX_LINK_CANDIDATES_PER_ROW).map(|index| format!("file-{index}.txt")).collect::<Vec<_>>();
    for name in &names {
        std::fs::write(root.join(name), b"source").expect("후보 파일 생성");
    }
    let candidates = [names[0].clone(), "missing.txt".to_string(), "src/main.rs".to_string()];

    let ordered = resolve_link_candidates(&projects, &root.to_string_lossy(), &candidates);
    let limited = resolve_link_candidates(&projects, &root.to_string_lossy(), &names);
    let outside = resolve_link_candidates(&projects, &outside_dir.to_string_lossy(), &["secret.txt".to_string()]);

    assert!(ordered[0].as_deref().is_some_and(|path| path.ends_with("file-0.txt")));
    assert_eq!(ordered[1], None);
    assert!(ordered[2].as_deref().is_some_and(|path| path.ends_with("main.rs")));
    assert!(limited[..MAX_LINK_CANDIDATES_PER_ROW].iter().all(Option::is_some));
    assert_eq!(limited[MAX_LINK_CANDIDATES_PER_ROW], None);
    assert_eq!(outside, vec![None]);
    std::fs::remove_dir_all(fixture).expect("fixture 정리");
}
