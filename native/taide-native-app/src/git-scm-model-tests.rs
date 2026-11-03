use taide_model::git::{GitChangeKind, StatusRow};
use taide_native_ui::intents::git::Section;

use super::{
    Collapsed, CommitGate, Direction, FocusTarget, ListRow, RowAction, SECTIONS, build_sections,
    commit_gate, discard_description, focus_target, header_indexes, list_rows, next_item_index,
    resolve_selection, row_key, row_menu, row_tooltip, split_path, status_color_key, status_letter,
    sticky_header_index,
};

const STASH_COUNT: usize = 2;
const GRAPH_COUNT: usize = 3;

fn row(path: &str, staged: Option<GitChangeKind>, unstaged: Option<GitChangeKind>) -> StatusRow {
    StatusRow {
        path: path.into(),
        abs_path: format!("/synthetic/{path}"),
        orig_path: None,
        orig_abs_path: None,
        staged,
        unstaged,
        is_conflicted: false,
    }
}

fn conflicted(path: &str) -> StatusRow {
    StatusRow {
        is_conflicted: true,
        ..row(
            path,
            Some(GitChangeKind::Modified),
            Some(GitChangeKind::Modified),
        )
    }
}

fn defaults() -> Collapsed {
    Collapsed::from_ids(&["stashes".to_owned()])
}

fn paths(sections: &super::Sections<'_>, section: Section) -> Vec<(String, GitChangeKind)> {
    sections
        .rows(section)
        .iter()
        .map(|change| (change.row.path.clone(), change.kind))
        .collect()
}

#[test]
fn 행을_충돌_스테이지_미스테이지_그룹으로_나누고_한_파일이_두_그룹에_들어갈_수_있다() {
    let rows = [
        conflicted("merge.txt"),
        row("staged.txt", Some(GitChangeKind::Added), None),
        row(
            "both.txt",
            Some(GitChangeKind::Modified),
            Some(GitChangeKind::Deleted),
        ),
        row("worktree.txt", None, Some(GitChangeKind::Untracked)),
        row("second.txt", None, Some(GitChangeKind::TypeChange)),
    ];
    let sections = build_sections(&rows, 0, 0, defaults());
    assert_eq!(
        paths(&sections, Section::Merge),
        [("merge.txt".to_owned(), GitChangeKind::Conflicted)]
    );
    assert_eq!(
        paths(&sections, Section::Staged),
        [
            ("staged.txt".to_owned(), GitChangeKind::Added),
            ("both.txt".to_owned(), GitChangeKind::Modified),
        ]
    );
    assert_eq!(
        paths(&sections, Section::Changes),
        [
            ("both.txt".to_owned(), GitChangeKind::Deleted),
            ("worktree.txt".to_owned(), GitChangeKind::Untracked),
            ("second.txt".to_owned(), GitChangeKind::TypeChange),
        ]
    );
    assert_eq!(
        SECTIONS.map(|section| sections.state(section).count),
        [1, 2, 3, 0, 0]
    );
    assert!(!sections.shows_no_changes);
}

#[test]
fn 빈_섹션은_숨기고_접힌_섹션도_개수를_보고하며_세_그룹이_비면_변경사항_없음이다() {
    let empty = build_sections(&[], 0, 0, defaults());
    assert!(empty.shows_no_changes);
    assert!(
        SECTIONS
            .iter()
            .all(|section| !empty.state(*section).is_visible)
    );
    assert!(list_rows(&empty).is_empty());

    let untouched = [row("clean.txt", None, None)];
    assert!(build_sections(&untouched, 0, 0, defaults()).shows_no_changes);

    let only_side = build_sections(&[], STASH_COUNT, GRAPH_COUNT, defaults());
    assert!(only_side.shows_no_changes);
    assert!(only_side.state(Section::Stashes).is_visible);
    assert!(only_side.state(Section::Graph).is_visible);
    assert_eq!(only_side.state(Section::Stashes).count, STASH_COUNT);
    assert!(only_side.state(Section::Stashes).is_collapsed);
    assert!(!only_side.state(Section::Graph).is_collapsed);

    let rows = [row("staged.txt", Some(GitChangeKind::Added), None)];
    let collapsed = build_sections(&rows, 0, 0, defaults().toggled(Section::Staged));
    let staged = collapsed.state(Section::Staged);
    assert!(staged.is_visible && staged.is_collapsed);
    assert_eq!(staged.count, 1);
    assert_eq!(list_rows(&collapsed), [ListRow::Header(Section::Staged)]);
    assert!(!collapsed.state(Section::Changes).is_visible);
}

#[test]
fn 목록은_merge_staged_changes_순서로_머리말과_펼친_그룹의_행을_잇는다() {
    let rows = [
        row("a.txt", None, Some(GitChangeKind::Modified)),
        conflicted("c.txt"),
        row(
            "b.txt",
            Some(GitChangeKind::Renamed),
            Some(GitChangeKind::Modified),
        ),
    ];
    let sections = build_sections(&rows, STASH_COUNT, GRAPH_COUNT, defaults());
    assert_eq!(
        list_rows(&sections),
        [
            ListRow::Header(Section::Merge),
            ListRow::Row(Section::Merge, 0),
            ListRow::Header(Section::Staged),
            ListRow::Row(Section::Staged, 0),
            ListRow::Header(Section::Changes),
            ListRow::Row(Section::Changes, 0),
            ListRow::Row(Section::Changes, 1),
        ]
    );
}

#[test]
fn 접힘_목록은_모르는_id를_무시하고_렌더_순서로_다시_쓰며_빈_목록은_아무것도_접지_않는다() {
    assert_eq!(defaults().ids(), ["stashes"]);
    assert_eq!(Collapsed::from_ids(&[]).ids(), Vec::<String>::new());
    let stored = ["graph", "future", "merge", "staged"].map(String::from);
    let collapsed = Collapsed::from_ids(&stored);
    assert_eq!(collapsed.ids(), ["merge", "staged", "graph"]);
    assert_eq!(Collapsed::from_ids(&collapsed.ids()), collapsed);
    assert!(collapsed.is_collapsed(Section::Graph));
    assert!(!collapsed.is_collapsed(Section::Stashes));
    let toggled = collapsed.toggled(Section::Changes).toggled(Section::Merge);
    assert_eq!(toggled.ids(), ["staged", "changes", "graph"]);
}

#[test]
fn 상태_글자와_색_토큰은_일곱_종류를_원본대로_매핑한다() {
    let expected = [
        (GitChangeKind::Modified, "M", "git.modified"),
        (GitChangeKind::Added, "A", "git.added"),
        (GitChangeKind::Deleted, "D", "git.deleted"),
        (GitChangeKind::Renamed, "R", "git.renamed"),
        (GitChangeKind::Untracked, "U", "git.untracked"),
        (GitChangeKind::TypeChange, "T", "git.modified"),
        (GitChangeKind::Conflicted, "!", "git.conflicted"),
    ];
    for (kind, letter, key) in expected {
        assert_eq!(status_letter(kind), letter, "{kind:?}");
        assert_eq!(status_color_key(kind), key, "{kind:?}");
    }
}

#[test]
fn 행_tooltip은_rename이면_원래_경로에서_화살표로_잇고_경로는_파일명과_디렉터리로_나눈다() {
    let mut renamed = row("src/new.rs", Some(GitChangeKind::Renamed), None);
    renamed.orig_path = Some("src/old.rs".into());
    assert_eq!(row_tooltip(&renamed), "src/old.rs → src/new.rs");
    assert_eq!(
        row_tooltip(&row("README.md", None, Some(GitChangeKind::Modified))),
        "README.md"
    );
    assert_eq!(
        split_path("src/deep/file.rs"),
        ("file.rs", Some("src/deep"))
    );
    assert_eq!(split_path("file.rs"), ("file.rs", None));
}

fn keys(sections: &super::Sections<'_>, list: &[ListRow]) -> Vec<Option<(Section, String)>> {
    list.iter()
        .map(|item| row_key(sections, *item).map(|key| (key.section, key.path)))
        .collect()
}

#[test]
fn 평탄화_행은_그룹과_경로를_키로_가지고_같은_경로도_그룹별로_다르며_머리말_위치를_모은다() {
    let rows = [
        conflicted("m.ts"),
        row("a.ts", Some(GitChangeKind::Modified), None),
        row(
            "b.ts",
            Some(GitChangeKind::Added),
            Some(GitChangeKind::Modified),
        ),
        row("c.ts", None, Some(GitChangeKind::Untracked)),
    ];
    let sections = build_sections(&rows, 0, 0, defaults());
    let list = list_rows(&sections);
    let staged = |path: &str| Some((Section::Staged, path.to_owned()));
    let changes = |path: &str| Some((Section::Changes, path.to_owned()));
    assert_eq!(
        keys(&sections, &list),
        [
            None,
            Some((Section::Merge, "m.ts".to_owned())),
            None,
            staged("a.ts"),
            staged("b.ts"),
            None,
            changes("b.ts"),
            changes("c.ts"),
        ]
    );
    assert_eq!(header_indexes(&list), [0, 2, 5]);
    assert!(header_indexes(&list_rows(&build_sections(&[], 0, 0, defaults()))).is_empty());
}

#[test]
fn sticky_머리말은_첫_보이는_행_이상에서_가장_가까운_머리말이다() {
    let headers = [0, 2, 5];
    for (first, expected) in [(0, 0), (1, 0), (2, 2), (4, 2), (5, 5), (9, 5)] {
        assert_eq!(
            sticky_header_index(&headers, first),
            Some(expected),
            "{first}"
        );
    }
    assert_eq!(sticky_header_index(&[], 3), None);
}

#[test]
fn 화살표_이동은_밖에서_양끝으로_들어오고_양끝에서_멈추며_스태시_머리말까지_이어진다() {
    use Direction::{Down, Up};
    assert_eq!(next_item_index(Down, None, 0), None);
    assert_eq!(next_item_index(Up, Some(2), 0), None);
    assert_eq!(next_item_index(Down, None, 3), Some(0));
    assert_eq!(next_item_index(Up, None, 3), Some(2));
    assert_eq!(next_item_index(Down, Some(0), 3), Some(1));
    assert_eq!(next_item_index(Up, Some(2), 3), Some(1));
    assert_eq!(next_item_index(Down, Some(2), 3), Some(2));
    assert_eq!(next_item_index(Up, Some(0), 3), Some(0));
    assert_eq!(next_item_index(Down, Some(0), 5), Some(1));
    assert_eq!(next_item_index(Down, Some(2), 5), Some(3));
    assert_eq!(next_item_index(Up, Some(3), 5), Some(2));
    let change_list = 4;
    let with_stash = change_list + 1;
    assert_eq!(
        next_item_index(Down, Some(change_list - 1), with_stash),
        Some(4)
    );
    assert_eq!(next_item_index(Down, Some(4), with_stash), Some(4));
    assert_eq!(next_item_index(Up, Some(4), with_stash), Some(3));
}

#[test]
fn 그래프_머리말은_목록_끝_다음_정거장이고_밖에서_위로는_그래프로_들어온다() {
    use Direction::{Down, Up};
    use FocusTarget::{Graph, Item};
    let count = 3;
    assert_eq!(
        focus_target(Down, Some(Item(0)), count, true),
        Some(Item(1))
    );
    assert_eq!(focus_target(Up, Some(Item(2)), count, true), Some(Item(1)));
    assert_eq!(focus_target(Up, Some(Item(0)), count, true), Some(Item(0)));
    assert_eq!(focus_target(Down, Some(Item(2)), count, true), Some(Graph));
    assert_eq!(
        focus_target(Down, Some(Item(2)), count, false),
        Some(Item(2))
    );
    assert_eq!(focus_target(Up, Some(Graph), count, true), Some(Item(2)));
    assert_eq!(focus_target(Down, Some(Graph), count, true), None);
    assert_eq!(focus_target(Down, None, count, true), Some(Item(0)));
    assert_eq!(focus_target(Up, None, count, true), Some(Graph));
    assert_eq!(focus_target(Up, None, count, false), Some(Item(2)));
    assert_eq!(focus_target(Down, None, 0, true), Some(Graph));
    assert_eq!(focus_target(Up, None, 0, true), Some(Graph));
    assert_eq!(focus_target(Up, Some(Graph), 0, true), None);
    assert_eq!(focus_target(Down, None, 0, false), None);
    assert_eq!(focus_target(Up, None, 0, false), None);
}

#[test]
fn 목록_선택은_일반_토글_범위_추가범위를_원본대로_계산하고_사라진_항목을_뺀다() {
    let ids = ["a", "b", "c", "d"];
    let plain = resolve_selection(&ids, &["a", "c"], Some(&"a"), &"b", false, false);
    assert_eq!((plain.selected, plain.anchor), (vec!["b"], "b"));

    let added = resolve_selection(&ids, &["a"], Some(&"a"), &"c", false, true);
    assert_eq!(added.selected, ["a", "c"]);
    let removed = resolve_selection(
        &ids,
        &added.selected,
        Some(&added.anchor),
        &"c",
        false,
        true,
    );
    assert_eq!((removed.selected, removed.anchor), (vec!["a"], "c"));

    let range = resolve_selection(&ids, &["a"], Some(&"b"), &"d", true, false);
    assert_eq!((range.selected, range.anchor), (vec!["b", "c", "d"], "b"));

    let extended = resolve_selection(&ids, &["a"], Some(&"c"), &"d", true, true);
    assert_eq!(extended.selected, ["a", "c", "d"]);

    let gone = resolve_selection(&ids, &["gone", "a"], Some(&"gone"), &"c", true, false);
    assert_eq!((gone.selected, gone.anchor), (vec!["c"], "c"));

    let unknown = resolve_selection(&ids, &["a", "gone"], Some(&"a"), &"x", false, false);
    assert_eq!((unknown.selected, unknown.anchor), (vec!["a"], "x"));
}

#[test]
fn 행_메뉴는_그룹마다_원본_순서와_구분선을_가진다() {
    use RowAction::{CopyPath, Discard, OpenChanges, OpenFile, Reveal, Stage, Unstage};
    assert_eq!(
        row_menu(Section::Merge),
        [
            Some(OpenFile),
            Some(OpenChanges),
            None,
            Some(CopyPath),
            Some(Reveal)
        ]
    );
    assert_eq!(
        row_menu(Section::Staged),
        [
            Some(OpenFile),
            Some(OpenChanges),
            Some(Unstage),
            None,
            Some(CopyPath),
            Some(Reveal)
        ]
    );
    assert_eq!(
        row_menu(Section::Changes),
        [
            Some(OpenFile),
            Some(OpenChanges),
            Some(Stage),
            Some(Discard),
            None,
            Some(CopyPath),
            Some(Reveal)
        ]
    );
    assert!(row_menu(Section::Stashes).is_empty());
    let labels: Vec<_> = [
        OpenFile,
        OpenChanges,
        Stage,
        Unstage,
        Discard,
        CopyPath,
        Reveal,
    ]
    .map(RowAction::label)
    .into();
    assert_eq!(
        labels,
        [
            "git.openFile",
            "git.openChanges",
            "git.stageChanges",
            "git.unstageChanges",
            "git.discard",
            "explorer.copyPath",
            "explorer.reveal"
        ]
    );
}

#[test]
fn discard_문구는_한_개면_그_경로_여러_개면_파일_수다() {
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: std::collections::BTreeMap::from([
            (
                "git.discardDescription".to_owned(),
                "This reverts changes in {{target}}.".to_owned(),
            ),
            ("git.fileCount".to_owned(), "{{count}} files".to_owned()),
        ]),
    };
    assert_eq!(
        discard_description(&locale, &["src/a.rs".to_owned()]),
        "This reverts changes in src/a.rs."
    );
    assert_eq!(
        discard_description(&locale, &["a".to_owned(), "b".to_owned()]),
        "This reverts changes in 2 files."
    );
}

#[test]
fn 커밋_gate는_충돌이면_막고_스테이지_없이_미스테이지만_있으면_전체_스테이지를_확인한다() {
    let modified = Some(GitChangeKind::Modified);
    let cases = [
        (vec![], CommitGate::Commit),
        (vec![row("a", modified, None)], CommitGate::Commit),
        (vec![row("a", None, modified)], CommitGate::ConfirmStageAll),
        (
            vec![row("a", modified, None), row("b", None, modified)],
            CommitGate::Commit,
        ),
        (
            vec![row("a", modified, modified), row("b", None, modified)],
            CommitGate::Commit,
        ),
        (vec![conflicted("c")], CommitGate::BlockedByConflicts),
        (
            vec![row("a", modified, None), conflicted("c")],
            CommitGate::BlockedByConflicts,
        ),
        (
            vec![row("a", None, modified), conflicted("c")],
            CommitGate::BlockedByConflicts,
        ),
    ];
    for (rows, gate) in cases {
        assert_eq!(commit_gate(&rows), gate, "{rows:?}");
    }
}
