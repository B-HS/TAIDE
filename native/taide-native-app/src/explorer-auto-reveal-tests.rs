use taide_model::tree::{TreeEntryKind, TreeRow};

use super::{Decision, Input, decide};

const ACTIVE_PATH: &str = "/project/src/main.ts";
const OTHER_PATH: &str = "/project/src/other.ts";

fn rows(paths: &[&str]) -> Vec<TreeRow> {
    paths
        .iter()
        .map(|path| TreeRow {
            path: (*path).to_owned(),
            name: path.rsplit('/').next().unwrap_or_default().to_owned(),
            kind: TreeEntryKind::File,
            depth: 1,
            expanded: false,
            has_children: false,
        })
        .collect()
}

fn input(rows: &[TreeRow]) -> Input<'_> {
    Input {
        is_enabled: true,
        active_path: Some(ACTIVE_PATH),
        rows,
        is_sidebar_visible: true,
        is_files_view: true,
        last_revealed: None,
    }
}

#[test]
fn 설정이_꺼져_있으면_skip한다() {
    let rows = rows(&[]);
    let decision = decide(Input {
        is_enabled: false,
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn 활성_파일_경로가_없으면_skip한다() {
    let rows = rows(&[]);
    let decision = decide(Input {
        active_path: None,
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn 사이드바가_보이지_않으면_skip한다() {
    let rows = rows(&[]);
    let decision = decide(Input {
        is_sidebar_visible: false,
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn 사이드바가_files_보기가_아니면_skip한다() {
    let rows = rows(&[]);
    let decision = decide(Input {
        is_files_view: false,
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn 직전에_이미_reveal한_경로면_skip한다() {
    let rows = rows(&[]);
    let decision = decide(Input {
        last_revealed: Some(ACTIVE_PATH),
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn 행이_이미_있으면_select_only다() {
    let rows = rows(&[ACTIVE_PATH]);
    assert_eq!(decide(input(&rows)), Decision::SelectOnly);
}

#[test]
fn 행이_없으면_reveal_then_select다() {
    let rows = rows(&[OTHER_PATH]);
    assert_eq!(decide(input(&rows)), Decision::RevealThenSelect);
}

#[test]
fn 행이_있어도_사이드바가_닫혀_있으면_skip이_우선한다() {
    let rows = rows(&[ACTIVE_PATH]);
    let decision = decide(Input {
        is_sidebar_visible: false,
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn 직전_reveal_경로와_다른_파일로_전환하면_다시_reveal한다() {
    let rows = rows(&[]);
    let decision = decide(Input {
        last_revealed: Some(OTHER_PATH),
        ..input(&rows)
    });
    assert_eq!(decision, Decision::RevealThenSelect);
}

#[test]
fn 설정이_꺼져_있으면_행이_있어도_select_only조차_하지_않는다() {
    let rows = rows(&[ACTIVE_PATH]);
    let decision = decide(Input {
        is_enabled: false,
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn files_보기가_아니면_행이_있어도_skip한다() {
    let rows = rows(&[ACTIVE_PATH]);
    let decision = decide(Input {
        is_files_view: false,
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}

#[test]
fn 행이_하나도_없으면_reveal_then_select다() {
    let rows = rows(&[]);
    assert_eq!(decide(input(&rows)), Decision::RevealThenSelect);
}

#[test]
fn 직전_reveal_경로와_같으면_행이_있어도_select_only를_반복하지_않는다() {
    let rows = rows(&[ACTIVE_PATH]);
    let decision = decide(Input {
        last_revealed: Some(ACTIVE_PATH),
        ..input(&rows)
    });
    assert_eq!(decision, Decision::Skip);
}
