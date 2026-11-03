use std::collections::HashMap;

use taide_model::git::{GitChangeKind, GitStatus, StatusRow};
use taide_model::ids::ProjectId;

use super::{Decoration, GitDecorations, build};

const ROOT: &str = "/repo";

fn row(abs_path: &str) -> StatusRow {
    StatusRow {
        path: abs_path
            .strip_prefix(&format!("{ROOT}/"))
            .unwrap_or(abs_path)
            .to_owned(),
        abs_path: abs_path.to_owned(),
        orig_path: None,
        orig_abs_path: None,
        staged: None,
        unstaged: None,
        is_conflicted: false,
    }
}

fn unstaged(abs_path: &str, kind: GitChangeKind) -> StatusRow {
    StatusRow {
        unstaged: Some(kind),
        ..row(abs_path)
    }
}

fn status(rows: Vec<StatusRow>) -> GitStatus {
    GitStatus {
        rows,
        branch: None,
        ahead: 0,
        behind: 0,
        has_remote: false,
    }
}

#[test]
fn unstaged_변경_종류를_파일_상태로_매핑한다() {
    let map = build(
        &[unstaged("/repo/src/a.ts", GitChangeKind::Modified)],
        Some(ROOT),
    );
    assert_eq!(map.get("/repo/src/a.ts"), Some(&Decoration::Modified));
}

#[test]
fn untracked와_type_change를_각각_untracked_modified로_매핑한다() {
    let map = build(
        &[
            unstaged("/repo/new.ts", GitChangeKind::Untracked),
            unstaged("/repo/link.ts", GitChangeKind::TypeChange),
        ],
        Some(ROOT),
    );
    assert_eq!(map.get("/repo/new.ts"), Some(&Decoration::Untracked));
    assert_eq!(map.get("/repo/link.ts"), Some(&Decoration::Modified));
}

#[test]
fn staged_added는_unstaged_modified보다_우선한다() {
    let map = build(
        &[StatusRow {
            staged: Some(GitChangeKind::Added),
            ..unstaged("/repo/a.ts", GitChangeKind::Modified)
        }],
        Some(ROOT),
    );
    assert_eq!(map.get("/repo/a.ts"), Some(&Decoration::Added));
}

#[test]
fn is_conflicted는_변경_종류와_무관하게_conflicted다() {
    let map = build(
        &[StatusRow {
            staged: Some(GitChangeKind::Added),
            is_conflicted: true,
            ..unstaged("/repo/a.ts", GitChangeKind::Modified)
        }],
        Some(ROOT),
    );
    assert_eq!(map.get("/repo/a.ts"), Some(&Decoration::Conflicted));
}

#[test]
fn 변경_종류가_없고_비충돌인_행은_등재하지_않는다() {
    assert!(build(&[row("/repo/a.ts")], Some(ROOT)).is_empty());
}

#[test]
fn 조상_디렉터리로_전파하되_프로젝트_루트_자신은_제외한다() {
    let map = build(
        &[unstaged("/repo/src/deep/a.ts", GitChangeKind::Modified)],
        Some(ROOT),
    );
    assert_eq!(map.get("/repo/src/deep"), Some(&Decoration::Modified));
    assert_eq!(map.get("/repo/src"), Some(&Decoration::Modified));
    assert!(!map.contains_key("/repo"));
    assert!(!map.contains_key("/"));
}

#[test]
fn 디렉터리는_자식들_중_최우선_상태를_갖는다() {
    let map = build(
        &[
            unstaged("/repo/src/a.ts", GitChangeKind::Untracked),
            StatusRow {
                is_conflicted: true,
                ..row("/repo/src/b.ts")
            },
        ],
        Some(ROOT),
    );
    assert_eq!(map.get("/repo/src"), Some(&Decoration::Conflicted));
}

#[test]
fn 형제_디렉터리의_접두사_충돌은_전파되지_않는다() {
    let map = build(
        &[unstaged("/repo-other/a.ts", GitChangeKind::Modified)],
        Some(ROOT),
    );
    assert!(!map.contains_key("/repo"));
    assert_eq!(map.get("/repo-other/a.ts"), Some(&Decoration::Modified));
}

#[test]
fn 프로젝트_루트가_없으면_빈_맵이다() {
    assert!(build(&[unstaged("/repo/a.ts", GitChangeKind::Modified)], None).is_empty());
}

#[test]
fn 입력_저장소는_마운트된_루트로_파생하고_none이면_지운다() {
    let project = ProjectId::new();
    let mut decorations = GitDecorations::default();
    let changed = status(vec![unstaged("/repo/src/new.rs", GitChangeKind::Untracked)]);
    decorations.set(&project, Some(&changed));
    assert_eq!(decorations.get(&project).map(HashMap::len), Some(0));
    decorations.mount(HashMap::from([(project.clone(), ROOT.to_owned())]));
    assert_eq!(decorations.mounted_projects(), vec![project.clone()]);
    let paths = decorations.get(&project).unwrap();
    assert_eq!(paths.get("/repo/src/new.rs"), Some(&Decoration::Untracked));
    assert_eq!(paths.get("/repo/src"), Some(&Decoration::Untracked));
    decorations.set(&project, None);
    assert!(decorations.get(&project).is_none());
    decorations.set(&project, Some(&changed));
    assert_eq!(
        decorations.get(&project).unwrap().get("/repo/src/new.rs"),
        Some(&Decoration::Untracked)
    );
    decorations.forget(&project);
    assert!(decorations.get(&project).is_none());
    assert!(decorations.mounted_projects().is_empty());
}
