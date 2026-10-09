use std::ops::{Range, RangeInclusive};
use std::time::Instant;

use ropey::Rope;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::display_map::{DisplayMap, WrapSettings, WrappingIndent};
use taide_native_editor::document::{DocumentId, DocumentSnapshot, Edit, UndoGroup};
use taide_native_editor::editing::{
    Motion, delete_backward, delete_forward, insert_line_break, line_content_range,
    move_selection_displayed, type_text,
};
use taide_native_editor::folding::{
    FoldClick, FoldCommand, FoldRegion, FoldToggle, FoldingModel, MAX_FOLDING_REGIONS, click_fold,
    hidden_lines, indent_regions, reconcile_folds, reveal_carets, run_fold_command,
};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};

const DOCUMENT_LIMIT: usize = 1;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 16;
const BYTE_LIMIT: usize = 64 * 1024;
const LARGE_BYTE_LIMIT: usize = 16 * 1024 * 1024;
const LARGE_BLOCKS: usize = 10_000;
const LARGE_BLOCK_LINES: usize = 5;
const TAB_SIZE: u32 = 4;
const FULL_WIDTH_COLUMNS: f64 = 2.0;
const WRAP_COLUMN: u32 = 12;
const MONACO_REGION_LIMIT: usize = 5000;
const BLOCKS: &str = "fn a() {\n    let x = 1;\n\n    if x {\n        y();\n    }\n\n}\nfn b() {}";
const TABBED: &str = "a\n\tb\n  c\n\td\ne";
const MIXED_INDENT: &str = "a\n    b\n \tc\ne";
const WIDE_TAB_SIZE: u32 = 8;
const LIMITED: &str = "a\n  b\n    c\n      d\n  e\n    f\ng\n  h";
const LIMITED_REGIONS: [(usize, usize); 5] = [(0, 5), (1, 3), (2, 3), (4, 5), (6, 7)];
const NESTED: &str = concat!(
    "class A {\n",
    "    fn a() {\n",
    "        one();\n",
    "        two();\n",
    "    }\n",
    "    fn b() {\n",
    "        three();\n",
    "    }\n",
    "}\n",
    "tail"
);
const OUTER: FoldRegion = FoldRegion {
    start_line: 0,
    end_line: 7,
};
const FIRST: FoldRegion = FoldRegion {
    start_line: 1,
    end_line: 3,
};
const SECOND: FoldRegion = FoldRegion {
    start_line: 5,
    end_line: 6,
};
const TAIL_LINE: usize = 9;
const FLAT: &str = "head\n  one\n  two\nnext\nlast";
const FLAT_HIDDEN: (usize, usize) = (1, 2);
const WRAPPED_HEADER: &str = "alpha beta gamma delta\n  one\n  two\nnext";
const INDENT: IndentOptions = IndentOptions {
    tab_size: TAB_SIZE,
    insert_spaces: true,
};
const DOWN: Motion = Motion::Vertical {
    lines: 1,
    tab_size: TAB_SIZE,
};
const UP: Motion = Motion::Vertical {
    lines: -1,
    tab_size: TAB_SIZE,
};

#[test]
fn 구문_접기는_provider_우선과_중첩_종류_한도_및_문서_버전을_보존한다() {
    use taide_native_editor::syntax_folding::{SyntaxFoldRange, SyntaxFolds};
    let (mut store, view, document) = fixture(NESTED);
    let range = |start_line, end_line, kind: &str| SyntaxFoldRange {
        region: FoldRegion {
            start_line,
            end_line,
        },
        kind: Some(kind.into()),
    };
    let current = snapshot(&store, document);
    let syntax = SyntaxFolds::new(
        &current,
        vec![
            vec![
                range(5, 7, "imports"),
                range(0, 8, "region"),
                range(1, 4, "custom"),
                range(3, 6, "comment"),
                range(8, 8, "invalid"),
                range(8, 99, "invalid"),
            ],
            vec![range(0, 7, "comment"), range(6, 7, "comment")],
        ],
    );
    assert_eq!(
        syntax.regions().as_ref(),
        &[
            FoldRegion {
                start_line: 0,
                end_line: 8
            },
            FoldRegion {
                start_line: 1,
                end_line: 4
            },
            FoldRegion {
                start_line: 5,
                end_line: 7
            },
            FoldRegion {
                start_line: 6,
                end_line: 7
            },
        ]
    );
    assert_eq!(syntax.kind(0), Some("region"));
    assert_eq!(syntax.kind(1), Some("custom"));
    assert!(syntax.describes(&current));
    replace(&mut store, document, 0..0, "prefix\n");
    assert!(!syntax.describes(&snapshot(&store, document)));
    assert!(
        !taide_native_editor::folding::run_syntax_fold_command(
            &mut store,
            view,
            &syntax,
            FoldCommand::FoldAll
        )
        .unwrap()
    );
    let text = (0..LARGE_BLOCKS * LARGE_BLOCK_LINES)
        .map(|_| "line\n")
        .collect::<String>();
    let (store, _, document) = large_fixture(&text);
    let current = snapshot(&store, document);
    let providers = (0..2)
        .map(|offset| {
            (0..MAX_FOLDING_REGIONS)
                .map(|index| {
                    range(
                        (offset * MAX_FOLDING_REGIONS + index) * 2,
                        (offset * MAX_FOLDING_REGIONS + index) * 2 + 1,
                        "region",
                    )
                })
                .collect()
        })
        .collect();
    assert_eq!(
        SyntaxFolds::new(&current, providers).regions().len(),
        MAX_FOLDING_REGIONS
    );
}

#[test]
fn 구문_종류별_접기는_imports를_개별_반전하고_수동_범위를_보존한다() {
    use taide_native_editor::folding::run_syntax_fold_command;
    use taide_native_editor::syntax_folding::{SyntaxFoldRange, SyntaxFolds};
    let (mut store, view, document) = fixture(NESTED);
    let current = snapshot(&store, document);
    let syntax = SyntaxFolds::new(
        &current,
        vec![vec![
            SyntaxFoldRange {
                region: FIRST,
                kind: Some("imports".into()),
            },
            SyntaxFoldRange {
                region: SECOND,
                kind: Some("imports".into()),
            },
            SyntaxFoldRange {
                region: OUTER,
                kind: Some("comment".into()),
            },
        ]],
    );
    hide(&mut store, view, &[(FIRST.start_line + 1, FIRST.end_line)]);
    run_syntax_fold_command(&mut store, view, &syntax, FoldCommand::ToggleImports).unwrap();
    assert_eq!(
        hidden(&store, view),
        vec![(SECOND.start_line + 1, SECOND.end_line + 1)]
    );
    run_syntax_fold_command(&mut store, view, &syntax, FoldCommand::FoldAllBlockComments).unwrap();
    assert!(
        FoldingModel::new(
            syntax.regions(),
            &current,
            &store.views().get(view).unwrap().folds
        )
        .header(OUTER.start_line)
        .unwrap()
    );
    run_syntax_fold_command(&mut store, view, &syntax, FoldCommand::UnfoldAll).unwrap();
    place(
        &mut store,
        view,
        line_start(&current, 1),
        line_start(&current, 5),
    );
    run_fold_command(
        &mut store,
        view,
        syntax.regions(),
        FoldCommand::CreateFromSelection,
    )
    .unwrap();
    run_syntax_fold_command(&mut store, view, &syntax, FoldCommand::ToggleImports).unwrap();
    assert_eq!(
        store.views().get(view).unwrap().manual_folds,
        vec![fold(&current, 2..=4)]
    );
    assert!(
        store
            .views()
            .get(view)
            .unwrap()
            .folds
            .contains(&fold(&current, 2..=4))
    );
}

#[test]
fn 수동_범위는_끝열1을_제외하고_펼친뒤_편집과_다중뷰를_따른다() {
    let (mut store, view, document) = fixture(NESTED);
    let current = snapshot(&store, document);
    let other = store
        .attach_view(
            ViewKey {
                window: "other".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    place(
        &mut store,
        view,
        line_start(&current, 1),
        line_start(&current, 5),
    );
    run_fold_command(&mut store, view, &[], FoldCommand::CreateFromSelection).unwrap();
    assert_eq!(
        store.views().get(view).unwrap().manual_folds,
        vec![fold(&current, 2..=4)]
    );
    assert!(store.views().get(other).unwrap().manual_folds.is_empty());
    assert_eq!(
        caret(&store, view),
        (line_start(&current, 1), line_start(&current, 1))
    );
    run_fold_command(&mut store, view, &[], FoldCommand::UnfoldAll).unwrap();
    assert!(store.views().get(view).unwrap().folds.is_empty());
    replace(&mut store, document, 0..0, "prefix\n");
    let edited = snapshot(&store, document);
    let state = store.views().get(view).unwrap();
    assert_eq!(state.manual_folds, vec![fold(&edited, 3..=5)]);
    assert_eq!(
        FoldingModel::with_manual(&[], &edited, &state.folds, &state.manual_folds).header(2),
        Some(false)
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store.views().get(view).unwrap().manual_folds,
        vec![fold(&current, 2..=4)]
    );
    assert!(store.redo(document).unwrap());
    assert_eq!(
        store.views().get(view).unwrap().manual_folds,
        vec![fold(&edited, 3..=5)]
    );
    place_line(&mut store, view, 2);
    run_fold_command(&mut store, view, &[], FoldCommand::Fold).unwrap();
    assert_eq!(hidden(&store, view), vec![(3, 6)]);
    run_fold_command(&mut store, view, &[], FoldCommand::RemoveManualRanges).unwrap();
    assert!(store.views().get(view).unwrap().manual_folds.is_empty());
    assert!(store.views().get(view).unwrap().folds.is_empty());
}

#[test]
fn 수동_범위는_기존_교차_접기보다_우선하고_모든_선택의_시작을_보존한다() {
    let (mut store, view, document) = fixture(NESTED);
    let current = snapshot(&store, document);
    let provider = FoldRegion {
        start_line: 0,
        end_line: 3,
    };
    hide(&mut store, view, &[(1, 3)]);
    let state = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection {
                        anchor: line_start(&current, 2),
                        head: line_end(&current, 5),
                    },
                    Selection {
                        anchor: line_start(&current, 6),
                        head: line_end(&current, 8),
                    },
                ],
            },
            state.scroll,
            state.folds,
        )
        .unwrap();
    run_fold_command(
        &mut store,
        view,
        &[provider],
        FoldCommand::CreateFromSelection,
    )
    .unwrap();
    let state = store.views().get(view).unwrap();
    let model = FoldingModel::with_manual(&[provider], &current, &state.folds, &state.manual_folds);
    assert_eq!(
        model.regions(),
        &[
            FoldRegion {
                start_line: 2,
                end_line: 5
            },
            FoldRegion {
                start_line: 6,
                end_line: 8
            },
        ]
    );
    assert_eq!(state.selection.primary, 1);
    assert_eq!(
        state
            .selection
            .selections
            .iter()
            .map(|selection| selection.head)
            .collect::<Vec<_>>(),
        vec![line_start(&current, 2), line_start(&current, 6)]
    );
}

#[test]
fn 수동_범위는_빈_본문_한_줄도_접는다() {
    let (mut store, view, document) = fixture("header\n\ntail");
    let current = snapshot(&store, document);
    place(&mut store, view, 0, line_start(&current, 2));
    run_fold_command(&mut store, view, &[], FoldCommand::CreateFromSelection).unwrap();
    assert_eq!(hidden(&store, view), vec![(1, 2)]);
    assert_eq!(store.views().get(view).unwrap().manual_folds.len(), 1);
    replace(&mut store, document, 0..0, "prefix\n");
    assert_eq!(hidden(&store, view), vec![(2, 3)]);
    run_fold_command(&mut store, view, &[], FoldCommand::UnfoldAll).unwrap();
    assert_eq!(store.views().get(view).unwrap().manual_folds.len(), 1);
    assert!(store.views().get(view).unwrap().folds.is_empty());
}

fn regions(text: &str, tab_size: u32, limit: usize) -> Vec<(usize, usize)> {
    indent_regions(&Rope::from_str(text), tab_size, limit)
        .into_iter()
        .map(|region| (region.start_line, region.end_line))
        .collect()
}

fn fixture(text: &str) -> (EditorStore, ViewId, DocumentId) {
    sized_fixture(text, BYTE_LIMIT)
}

fn large_fixture(text: &str) -> (EditorStore, ViewId, DocumentId) {
    sized_fixture(text, LARGE_BYTE_LIMIT)
}

fn sized_fixture(text: &str, byte_limit: usize) -> (EditorStore, ViewId, DocumentId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: byte_limit,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/folding.rs".into(),
            OpenedFile {
                path: "/synthetic/folding.rs".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, view, document)
}

fn snapshot(store: &EditorStore, document: DocumentId) -> DocumentSnapshot {
    store.documents().snapshot(document).unwrap()
}

fn line_start(document: &DocumentSnapshot, line: usize) -> usize {
    document.rope.line_to_byte(line)
}

fn line_end(document: &DocumentSnapshot, line: usize) -> usize {
    line_content_range(document, line).end
}

fn fold(document: &DocumentSnapshot, hidden: RangeInclusive<usize>) -> Range<usize> {
    line_start(document, *hidden.start())..line_end(document, *hidden.end())
}

fn hide(store: &mut EditorStore, view: ViewId, hidden: &[(usize, usize)]) {
    let current = store.views().get(view).unwrap().clone();
    let document = snapshot(store, current.document);
    store
        .set_view_state(
            view,
            current.selection,
            current.scroll,
            hidden
                .iter()
                .map(|(first, last)| fold(&document, *first..=*last))
                .collect(),
        )
        .unwrap();
}

fn hidden(store: &EditorStore, view: ViewId) -> Vec<(usize, usize)> {
    let current = store.views().get(view).unwrap();
    let document = snapshot(store, current.document);
    for fold in &current.folds {
        let first = document.rope.byte_to_line(fold.start);
        let last = document.rope.byte_to_line(fold.end);
        assert_eq!(
            *fold,
            line_start(&document, first)..line_end(&document, last)
        );
    }
    hidden_lines(&document.rope, &current.folds)
        .into_iter()
        .map(|lines| (lines.start, lines.end))
        .collect()
}

fn collapsed(store: &EditorStore, view: ViewId) -> Vec<(usize, usize)> {
    let current = store.views().get(view).unwrap();
    let document = snapshot(store, current.document);
    current
        .folds
        .iter()
        .map(|fold| {
            (
                document.rope.byte_to_line(fold.start) - 1,
                document.rope.byte_to_line(fold.end),
            )
        })
        .collect()
}

fn place(store: &mut EditorStore, view: ViewId, anchor: usize, head: usize) {
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor, head }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
}

fn place_line(store: &mut EditorStore, view: ViewId, line: usize) {
    let document = snapshot(store, store.views().get(view).unwrap().document);
    let start = line_start(&document, line);
    place(store, view, start, start);
}

fn caret(store: &EditorStore, view: ViewId) -> (usize, usize) {
    let selection = &store.views().get(view).unwrap().selection;
    assert_eq!(selection.selections.len(), 1);
    (selection.selections[0].anchor, selection.selections[0].head)
}

fn replace(store: &mut EditorStore, document: DocumentId, bytes: Range<usize>, text: &str) {
    let revision = snapshot(store, document).revision;
    store
        .apply(
            document,
            Transaction {
                revision,
                group: UndoGroup(revision),
                origin: None,
                selection_after: None,
                edits: vec![Edit {
                    bytes,
                    text: text.into(),
                }],
            },
        )
        .unwrap();
}

fn computed(store: &EditorStore, document: DocumentId) -> Vec<FoldRegion> {
    indent_regions(
        &snapshot(store, document).rope,
        TAB_SIZE,
        MAX_FOLDING_REGIONS,
    )
}

fn run(store: &mut EditorStore, view: ViewId, command: FoldCommand) -> bool {
    let regions = computed(store, store.views().get(view).unwrap().document);
    run_fold_command(store, view, &regions, command).unwrap()
}

fn click(store: &mut EditorStore, view: ViewId, line: usize, toggle: FoldToggle) -> bool {
    let regions = computed(store, store.views().get(view).unwrap().document);
    click_fold(
        store,
        view,
        &regions,
        FoldClick {
            line,
            is_on_control: true,
            toggle,
        },
    )
    .unwrap()
}

fn displayed(store: &EditorStore, view: ViewId, wrap_column: Option<u32>) -> DisplayMap {
    let current = store.views().get(view).unwrap();
    let document = snapshot(store, current.document);
    let mut map = DisplayMap::build(
        &document,
        wrap_column.map(|wrap_column| WrapSettings {
            wrap_column,
            tab_size: TAB_SIZE,
            full_width_columns: FULL_WIDTH_COLUMNS,
            wrapping_indent: WrappingIndent::Same,
        }),
    );
    map.set_hidden_lines(&hidden_lines(&document.rope, &current.folds));
    map
}

fn step(store: &mut EditorStore, view: ViewId, motion: Motion, extend: bool) {
    let display = displayed(store, view, None);
    move_selection_displayed(store, view, motion, extend, &display).unwrap();
}

#[test]
fn 들여쓰기_접기_영역은_더_깊은_줄을_가진_줄에서_시작해_같거나_얕은_다음_줄_앞에서_끝나고_사이의_빈_줄을_포함한다()
 {
    assert_eq!(MAX_FOLDING_REGIONS, MONACO_REGION_LIMIT);
    assert_eq!(
        regions(BLOCKS, TAB_SIZE, MAX_FOLDING_REGIONS),
        [(0, 6), (3, 4)]
    );
    for (text, expected) in [
        ("a\r\n  b\r\n \t \r\nc", vec![(0, 2)]),
        ("a\r  b\r\rc", vec![(0, 2)]),
        ("a\n  b\n\n", vec![(0, 3)]),
        ("a\nb\n  \nc", vec![]),
        ("  a\nb", vec![]),
        ("a\n  b", vec![(0, 1)]),
        ("", vec![]),
    ] {
        assert_eq!(
            regions(text, TAB_SIZE, MAX_FOLDING_REGIONS),
            expected,
            "{text:?}"
        );
    }
}

#[test]
fn 들여쓰기_깊이는_공백을_한_칸으로_탭을_다음_탭_정지로_세어_탭_크기에_따라_영역이_달라진다() {
    for (tab_size, expected) in [
        (4, vec![(0, 3), (2, 3)]),
        (2, vec![(0, 3)]),
        (1, vec![(0, 3), (1, 2)]),
        (0, vec![(0, 3), (1, 2)]),
    ] {
        assert_eq!(
            regions(TABBED, tab_size, MAX_FOLDING_REGIONS),
            expected,
            "tab {tab_size}"
        );
    }
    assert_eq!(
        regions(MIXED_INDENT, TAB_SIZE, MAX_FOLDING_REGIONS),
        [(0, 2)]
    );
    assert_eq!(
        regions(MIXED_INDENT, WIDE_TAB_SIZE, MAX_FOLDING_REGIONS),
        [(0, 2), (1, 2)]
    );
}

#[test]
fn 영역_수가_한도를_넘으면_얕은_들여쓰기부터_채우고_경계_깊이는_위에서부터_한도까지만_남긴다() {
    for (limit, kept) in [
        (LIMITED_REGIONS.len(), LIMITED_REGIONS.to_vec()),
        (4, vec![(0, 5), (1, 3), (4, 5), (6, 7)]),
        (3, vec![(0, 5), (1, 3), (6, 7)]),
        (1, vec![(0, 5)]),
        (0, vec![]),
    ] {
        assert_eq!(regions(LIMITED, 2, limit), kept, "limit {limit}");
    }
}

#[test]
fn 큰_문서의_들여쓰기_영역_계산과_접기_모델_합치기에_걸린_시간을_기록한다() {
    let text: String = (0..LARGE_BLOCKS)
        .map(|block| {
            format!("fn block_{block}() {{\n    if ready {{\n        work();\n    }}\n}}\n")
        })
        .collect();
    let (store, _, document) = large_fixture(&text);
    let snapshot = snapshot(&store, document);
    let started = Instant::now();
    let all = indent_regions(&snapshot.rope, TAB_SIZE, usize::MAX);
    let unlimited = started.elapsed();
    let started = Instant::now();
    let limited = indent_regions(&snapshot.rope, TAB_SIZE, MAX_FOLDING_REGIONS);
    let computing = started.elapsed();
    assert_eq!(all.len(), LARGE_BLOCKS * 2);
    assert_eq!(limited.len(), MAX_FOLDING_REGIONS);
    assert!(
        limited
            .iter()
            .all(|region| region.start_line % LARGE_BLOCK_LINES == 0)
    );
    let folds: Vec<Range<usize>> = limited
        .iter()
        .map(|region| fold(&snapshot, region.start_line + 1..=region.end_line))
        .collect();
    let started = Instant::now();
    let model = FoldingModel::new(&limited, &snapshot, &folds);
    let merging = started.elapsed();
    assert_eq!(model.folds(&snapshot.rope), folds);
    let started = Instant::now();
    let hidden = hidden_lines(&snapshot.rope, &folds);
    let hiding = started.elapsed();
    assert_eq!(hidden.len(), MAX_FOLDING_REGIONS);
    println!(
        "folding timing: lines {} regions {} unlimited {unlimited:?} limited {computing:?} merge {merging:?} hidden {hiding:?}",
        snapshot.rope.len_lines(),
        all.len(),
    );
}

#[test]
fn 접기_모델은_계산된_영역에_접힌_범위를_합치고_부모를_가진_영역_순서를_유지한다() {
    let (mut store, view, document) = fixture(NESTED);
    let regions = computed(&store, document);
    assert_eq!(regions, [OUTER, FIRST, SECOND]);
    hide(&mut store, view, &[(2, 3)]);
    let current = store.views().get(view).unwrap().clone();
    let snapshot = snapshot(&store, document);
    let model = FoldingModel::new(&regions, &snapshot, &current.folds);
    assert_eq!(model.regions(), [OUTER, FIRST, SECOND]);
    assert_eq!(
        [0, 1, 2].map(|index| model.is_collapsed(index)),
        [false, true, false]
    );
    assert_eq!(model.header(0), Some(false));
    assert_eq!(model.header(1), Some(true));
    assert_eq!(model.header(2), None);
    assert_eq!(model.header(5), Some(false));
    assert_eq!(model.folds(&snapshot.rope), current.folds);
    let recovered = FoldingModel::new(&regions, &snapshot, &[fold(&snapshot, 3..=3)]);
    assert_eq!(
        recovered.regions(),
        [
            OUTER,
            FIRST,
            FoldRegion {
                start_line: 2,
                end_line: 3
            },
            SECOND
        ]
    );
    assert_eq!(recovered.header(2), Some(true));
    let crossing = FoldingModel::new(&regions, &snapshot, &[fold(&snapshot, 3..=5)]);
    assert_eq!(crossing.regions(), [OUTER, FIRST, SECOND]);
    assert_eq!(crossing.folds(&snapshot.rope), []);
    let whole = 0..snapshot.rope.len_bytes();
    let beyond = FoldingModel::new(&[], &snapshot, std::slice::from_ref(&whole));
    assert!(beyond.regions().is_empty());
}

#[test]
fn 접기_명령은_캐럿_줄의_가장_안쪽_영역부터_접고_캐럿을_머리_줄_끝으로_옮기며_안쪽_접힘_상태를_보존한다()
 {
    let (mut store, view, document) = fixture(NESTED);
    let snapshot = snapshot(&store, document);
    place_line(&mut store, view, 2);
    assert!(run(&mut store, view, FoldCommand::Fold));
    assert_eq!(collapsed(&store, view), [(1, 3)]);
    assert_eq!(hidden(&store, view), [(2, 4)]);
    let header_end = line_end(&snapshot, 1);
    assert_eq!(caret(&store, view), (header_end, header_end));
    assert!(run(&mut store, view, FoldCommand::Fold));
    assert_eq!(collapsed(&store, view), [(0, 7), (1, 3)]);
    assert_eq!(hidden(&store, view), [(1, 8)]);
    let outer_end = line_end(&snapshot, 0);
    assert_eq!(caret(&store, view), (outer_end, outer_end));
    assert!(!run(&mut store, view, FoldCommand::Fold));
    assert!(run(&mut store, view, FoldCommand::Unfold));
    assert_eq!(collapsed(&store, view), [(1, 3)]);
    assert_eq!(caret(&store, view), (outer_end, outer_end));
    assert!(!run(&mut store, view, FoldCommand::Unfold));
    assert!(run(&mut store, view, FoldCommand::UnfoldRecursively));
    assert!(collapsed(&store, view).is_empty());
    assert!(run(&mut store, view, FoldCommand::FoldRecursively));
    assert_eq!(collapsed(&store, view), [(0, 7), (1, 3), (5, 6)]);
    assert!(run(&mut store, view, FoldCommand::UnfoldAll));
    assert!(collapsed(&store, view).is_empty());
    place_line(&mut store, view, TAIL_LINE);
    let tail = caret(&store, view);
    assert!(run(&mut store, view, FoldCommand::FoldAll));
    assert_eq!(collapsed(&store, view), [(0, 7), (1, 3), (5, 6)]);
    assert_eq!(caret(&store, view), tail);
    assert!(!run(&mut store, view, FoldCommand::FoldAll));
    assert!(!run(&mut store, view, FoldCommand::Fold));
    assert!(run(&mut store, view, FoldCommand::UnfoldAll));
}

#[test]
fn 전환과_나머지_접기_명령은_캐럿_줄의_영역을_기준으로_대상을_고른다() {
    let (mut store, view, _) = fixture(NESTED);
    place_line(&mut store, view, 6);
    assert!(run(&mut store, view, FoldCommand::ToggleFold));
    assert_eq!(collapsed(&store, view), [(5, 6)]);
    assert!(run(&mut store, view, FoldCommand::ToggleFold));
    assert!(collapsed(&store, view).is_empty());
    place_line(&mut store, view, 0);
    assert!(run(&mut store, view, FoldCommand::ToggleFoldRecursively));
    assert_eq!(collapsed(&store, view), [(0, 7), (1, 3), (5, 6)]);
    assert!(run(&mut store, view, FoldCommand::ToggleFoldRecursively));
    assert!(collapsed(&store, view).is_empty());
    place_line(&mut store, view, 2);
    assert!(run(&mut store, view, FoldCommand::FoldAllExcept));
    assert_eq!(collapsed(&store, view), [(5, 6)]);
    assert!(!run(&mut store, view, FoldCommand::FoldAllExcept));
    hide(&mut store, view, &[(2, 3), (6, 6)]);
    place_line(&mut store, view, 5);
    assert!(run(&mut store, view, FoldCommand::UnfoldAllExcept));
    assert_eq!(collapsed(&store, view), [(5, 6)]);
    place_line(&mut store, view, TAIL_LINE);
    for command in [
        FoldCommand::Fold,
        FoldCommand::Unfold,
        FoldCommand::ToggleFold,
        FoldCommand::FoldRecursively,
        FoldCommand::UnfoldRecursively,
        FoldCommand::ToggleFoldRecursively,
    ] {
        assert!(!run(&mut store, view, command), "{command:?}");
    }
}

#[test]
fn 이동_명령은_부모_이전_다음_영역의_머리_줄_시작에_캐럿_하나를_둔다() {
    let (mut store, view, document) = fixture(NESTED);
    let snapshot = snapshot(&store, document);
    for (from, command, target) in [
        (2, FoldCommand::GotoParentFold, Some(1)),
        (1, FoldCommand::GotoParentFold, Some(0)),
        (0, FoldCommand::GotoParentFold, None),
        (TAIL_LINE, FoldCommand::GotoParentFold, None),
        (1, FoldCommand::GotoNextFold, Some(5)),
        (5, FoldCommand::GotoNextFold, None),
        (2, FoldCommand::GotoNextFold, Some(5)),
        (0, FoldCommand::GotoNextFold, None),
        (TAIL_LINE, FoldCommand::GotoNextFold, None),
        (5, FoldCommand::GotoPreviousFold, Some(1)),
        (1, FoldCommand::GotoPreviousFold, None),
        (TAIL_LINE, FoldCommand::GotoPreviousFold, Some(5)),
        (3, FoldCommand::GotoPreviousFold, Some(1)),
    ] {
        let start = line_start(&snapshot, from);
        place(&mut store, view, start, line_end(&snapshot, from));
        assert_eq!(
            run(&mut store, view, command),
            target.is_some(),
            "{command:?} from {from}"
        );
        let expected = target.map_or((start, line_end(&snapshot, from)), |line| {
            (line_start(&snapshot, line), line_start(&snapshot, line))
        });
        assert_eq!(caret(&store, view), expected, "{command:?} from {from}");
        assert!(collapsed(&store, view).is_empty());
    }
}

#[test]
fn 머리_줄_클릭은_영역을_전환하고_재귀와_주변_전환은_안쪽과_무관한_영역을_먼저_다룬다() {
    let (mut store, view, _) = fixture(NESTED);
    place_line(&mut store, view, TAIL_LINE);
    assert!(click(&mut store, view, 1, FoldToggle::Region));
    assert_eq!(collapsed(&store, view), [(1, 3)]);
    assert!(click(&mut store, view, 1, FoldToggle::Region));
    assert!(collapsed(&store, view).is_empty());
    assert!(!click(&mut store, view, 2, FoldToggle::Region));
    assert!(!click(&mut store, view, TAIL_LINE, FoldToggle::Region));
    let text_end = |store: &mut EditorStore, line: usize| {
        let regions = computed(store, store.views().get(view).unwrap().document);
        click_fold(
            store,
            view,
            &regions,
            FoldClick {
                line,
                is_on_control: false,
                toggle: FoldToggle::Region,
            },
        )
        .unwrap()
    };
    assert!(!text_end(&mut store, 1));
    assert!(click(&mut store, view, 1, FoldToggle::Region));
    assert!(text_end(&mut store, 1));
    assert!(collapsed(&store, view).is_empty());
    assert!(click(&mut store, view, 0, FoldToggle::Recursive));
    assert_eq!(collapsed(&store, view), [(1, 3), (5, 6)]);
    assert!(click(&mut store, view, 0, FoldToggle::Recursive));
    assert_eq!(collapsed(&store, view), [(0, 7), (1, 3), (5, 6)]);
    assert!(click(&mut store, view, 0, FoldToggle::Recursive));
    assert!(collapsed(&store, view).is_empty());
    assert!(click(&mut store, view, 1, FoldToggle::Surrounding));
    assert_eq!(collapsed(&store, view), [(5, 6)]);
    assert!(click(&mut store, view, 1, FoldToggle::Surrounding));
    assert!(collapsed(&store, view).is_empty());
}

#[test]
fn 선택의_시작점이_숨김_줄에_들어가면_그_줄을_숨긴_접기를_모두_펼치고_끝점만_들어가면_유지한다() {
    let (mut store, view, document) = fixture(NESTED);
    let snapshot = snapshot(&store, document);
    hide(&mut store, view, &[(1, 7), (2, 3), (6, 6)]);
    assert!(!reveal_carets(&mut store, view).unwrap());
    place(&mut store, view, 0, line_start(&snapshot, 3));
    assert!(!reveal_carets(&mut store, view).unwrap());
    assert_eq!(collapsed(&store, view), [(0, 7), (1, 3), (5, 6)]);
    place_line(&mut store, view, 3);
    assert!(reveal_carets(&mut store, view).unwrap());
    assert_eq!(collapsed(&store, view), [(5, 6)]);
    assert_eq!(
        caret(&store, view),
        (line_start(&snapshot, 3), line_start(&snapshot, 3))
    );
    place(&mut store, view, line_start(&snapshot, 6), 0);
    assert!(reveal_carets(&mut store, view).unwrap());
    assert!(collapsed(&store, view).is_empty());
    assert!(!reveal_carets(&mut store, view).unwrap());
}

#[test]
fn 접힌_범위는_편집을_따라_옮겨지고_줄_경계로_맞춰지며_숨길_줄이_없어지면_버려진다() {
    let folded = |store: &mut EditorStore, view: ViewId| {
        hide(store, view, &[FLAT_HIDDEN]);
        store.views().get(view).unwrap().document
    };
    let (mut store, view, document) = fixture(FLAT);
    let original = snapshot(&store, document);
    folded(&mut store, view);
    replace(&mut store, document, 0..0, "top\n");
    assert_eq!(hidden(&store, view), [(2, 4)]);
    let header_end = line_end(&snapshot(&store, document), 1);
    replace(&mut store, document, header_end..header_end, " // 값");
    assert_eq!(hidden(&store, view), [(2, 4)]);
    let inside = line_end(&snapshot(&store, document), 2);
    replace(&mut store, document, inside - 1..inside, "x\n  y");
    assert_eq!(hidden(&store, view), [(2, 5)]);
    let after = line_end(&snapshot(&store, document), 5);
    replace(&mut store, document, after..after, "!");
    assert_eq!(hidden(&store, view), [(2, 5)]);

    let (mut store, view, document) = fixture(FLAT);
    folded(&mut store, view);
    let header_end = line_end(&original, 0);
    replace(&mut store, document, header_end..header_end, "\n  new");
    assert_eq!(hidden(&store, view), [(1, 4)]);

    let (mut store, view, document) = fixture(FLAT);
    folded(&mut store, view);
    replace(
        &mut store,
        document,
        header_end..line_start(&original, 1),
        "",
    );
    assert_eq!(hidden(&store, view), [(1, 2)]);

    let (mut store, view, document) = fixture(FLAT);
    folded(&mut store, view);
    replace(
        &mut store,
        document,
        line_end(&original, 2)..line_start(&original, 3),
        "",
    );
    assert_eq!(hidden(&store, view), [(1, 3)]);
    assert_eq!(
        snapshot(&store, document).rope.to_string(),
        "head\n  one\n  twonext\nlast"
    );

    let (mut store, view, document) = fixture(FLAT);
    folded(&mut store, view);
    replace(&mut store, document, header_end..line_end(&original, 2), "");
    assert!(store.views().get(view).unwrap().folds.is_empty());

    let (mut store, view, document) = fixture(FLAT);
    folded(&mut store, view);
    replace(
        &mut store,
        document,
        line_start(&original, 1)..line_end(&original, 2),
        "",
    );
    assert!(store.views().get(view).unwrap().folds.is_empty());
    assert_eq!(
        snapshot(&store, document).rope.to_string(),
        "head\n\nnext\nlast"
    );
}

#[test]
fn undo와_redo는_접힌_범위를_지우지_않고_되돌린_문서의_같은_줄로_옮긴다() {
    let (mut store, view, document) = fixture(FLAT);
    hide(&mut store, view, &[FLAT_HIDDEN]);
    let folds = store.views().get(view).unwrap().folds.clone();
    replace(&mut store, document, 0..0, "top\nmore\n");
    assert_eq!(hidden(&store, view), [(3, 5)]);
    let inside = line_start(&snapshot(&store, document), 4);
    replace(&mut store, document, inside..inside, "  extra\n");
    assert_eq!(hidden(&store, view), [(3, 6)]);
    assert!(store.undo(document).unwrap());
    assert_eq!(hidden(&store, view), [(3, 5)]);
    assert!(store.undo(document).unwrap());
    assert_eq!(store.views().get(view).unwrap().folds, folds);
    assert!(store.redo(document).unwrap());
    assert_eq!(hidden(&store, view), [(3, 5)]);
    assert!(store.redo(document).unwrap());
    assert_eq!(hidden(&store, view), [(3, 6)]);
}

#[test]
fn 편집_뒤_맞춤은_선택이_숨김_줄이나_늘어난_영역의_바로_뒤에_있으면_펼치고_아니면_새_영역_범위로_접어_둔다()
 {
    let (mut store, view, document) = fixture(NESTED);
    let reconcile = |store: &mut EditorStore| {
        let regions = computed(store, document);
        reconcile_folds(store, view, &regions).unwrap()
    };
    assert!(!reconcile(&mut store));
    hide(&mut store, view, &[(2, 3)]);
    place_line(&mut store, view, TAIL_LINE);
    type_text(&mut store, view, "x").unwrap();
    assert!(!reconcile(&mut store));
    assert_eq!(collapsed(&store, view), [(1, 3)]);

    let header_end = line_end(&snapshot(&store, document), 1);
    place(&mut store, view, header_end, header_end);
    insert_line_break(&mut store, view, INDENT).unwrap();
    assert_eq!(hidden(&store, view), [(2, 5)]);
    assert!(reconcile(&mut store));
    assert!(collapsed(&store, view).is_empty());

    let (mut store, view, document) = fixture(NESTED);
    hide(&mut store, view, &[(6, 6)]);
    let closing = line_start(&snapshot(&store, document), 7);
    replace(&mut store, document, closing..closing, "        four();\n");
    assert_eq!(collapsed(&store, view), [(5, 6)]);
    place_line(&mut store, view, TAIL_LINE + 1);
    assert!(reconcile(&mut store));
    assert_eq!(collapsed(&store, view), [(5, 7)]);
    let closing = line_start(&snapshot(&store, document), 8);
    replace(&mut store, document, closing..closing, "        five();\n");
    place_line(&mut store, view, 9);
    assert!(reconcile(&mut store));
    assert!(collapsed(&store, view).is_empty());

    let (mut store, view, document) = fixture(NESTED);
    hide(&mut store, view, &[(2, 3)]);
    place_line(&mut store, view, TAIL_LINE);
    let body = snapshot(&store, document);
    replace(
        &mut store,
        document,
        line_start(&body, 2)..line_end(&body, 3),
        "    one();\n    two();",
    );
    assert_eq!(computed(&store, document), [OUTER, SECOND]);
    assert!(!reconcile(&mut store));
    assert_eq!(collapsed(&store, view), [(1, 3)]);
}

#[test]
fn 접힌_줄_앞뒤의_삭제와_줄바꿈은_숨김_범위를_따라가고_캐럿이_숨김_줄에_남으면_펼쳐진다() {
    let (mut store, view, document) = fixture(FLAT);
    let original = snapshot(&store, document);
    hide(&mut store, view, &[FLAT_HIDDEN]);
    place_line(&mut store, view, 3);
    delete_backward(&mut store, view, INDENT).unwrap();
    assert_eq!(hidden(&store, view), [(1, 3)]);
    assert!(reveal_carets(&mut store, view).unwrap());
    assert!(store.views().get(view).unwrap().folds.is_empty());

    let (mut store, view, document) = fixture(FLAT);
    hide(&mut store, view, &[FLAT_HIDDEN]);
    let header_end = line_end(&original, 0);
    place(&mut store, view, header_end, header_end);
    delete_forward(&mut store, view).unwrap();
    assert_eq!(
        snapshot(&store, document).rope.to_string(),
        "head  one\n  two\nnext\nlast"
    );
    assert_eq!(hidden(&store, view), [(1, 2)]);
    assert!(!reveal_carets(&mut store, view).unwrap());
    type_text(&mut store, view, "!").unwrap();
    assert_eq!(hidden(&store, view), [(1, 2)]);
    assert!(!reveal_carets(&mut store, view).unwrap());
}

#[test]
fn 접힌_상태의_좌우_이동은_숨김_줄을_건너뛰고_세로_이동은_보이는_표시_줄만_지난다() {
    let (mut store, view, document) = fixture(FLAT);
    let snapshot = snapshot(&store, document);
    hide(&mut store, view, &[FLAT_HIDDEN]);
    let header_end = line_end(&snapshot, 0);
    let next = line_start(&snapshot, 3);
    place(&mut store, view, header_end, header_end);
    step(&mut store, view, Motion::Right, false);
    assert_eq!(caret(&store, view), (next, next));
    step(&mut store, view, Motion::Left, false);
    assert_eq!(caret(&store, view), (header_end, header_end));
    step(&mut store, view, Motion::Right, true);
    assert_eq!(caret(&store, view), (header_end, next));
    step(&mut store, view, Motion::Left, true);
    assert_eq!(caret(&store, view), (header_end, header_end));
    step(&mut store, view, DOWN, false);
    assert_eq!(
        caret(&store, view),
        (line_end(&snapshot, 3), line_end(&snapshot, 3))
    );
    step(&mut store, view, UP, false);
    assert_eq!(caret(&store, view), (header_end, header_end));
    assert!(!reveal_carets(&mut store, view).unwrap());

    place(&mut store, view, 0, line_start(&snapshot, 2));
    step(&mut store, view, Motion::Right, false);
    assert_eq!(caret(&store, view), (header_end, header_end));
    place(&mut store, view, 0, line_start(&snapshot, 2));
    step(&mut store, view, Motion::Right, true);
    assert_eq!(caret(&store, view), (0, next));
    place(&mut store, view, 0, line_start(&snapshot, 2));
    step(&mut store, view, DOWN, true);
    assert_eq!(caret(&store, view), (0, line_end(&snapshot, 3)));

    step(&mut store, view, Motion::DocumentEnd, false);
    assert!(!reveal_carets(&mut store, view).unwrap());
    hide(&mut store, view, &[FLAT_HIDDEN, (4, 4)]);
    place(
        &mut store,
        view,
        line_end(&snapshot, 3),
        line_end(&snapshot, 3),
    );
    step(&mut store, view, Motion::Right, false);
    assert_eq!(
        caret(&store, view),
        (line_end(&snapshot, 3), line_end(&snapshot, 3))
    );
    step(&mut store, view, Motion::DocumentEnd, false);
    assert!(reveal_carets(&mut store, view).unwrap());
    assert_eq!(hidden(&store, view), [(1, 3)]);
}

#[test]
fn 줄바꿈된_머리_줄의_마지막_표시_줄만_접힘_표식을_가지고_아래_이동은_숨김_뒤의_줄로_넘어간다() {
    let (mut store, view, document) = fixture(WRAPPED_HEADER);
    let snapshot = snapshot(&store, document);
    hide(&mut store, view, &[FLAT_HIDDEN]);
    let display = displayed(&store, view, Some(WRAP_COLUMN));
    let header_rows = display.rows_of_line(0);
    assert!(header_rows.len() > 1);
    assert_eq!(display.row_count(), header_rows.len() + 1);
    for row in header_rows.clone() {
        assert_eq!(
            display.segment(&snapshot, row).ends_folded,
            row + 1 == header_rows.end
        );
    }
    assert!(!display.segment(&snapshot, header_rows.end).ends_folded);
    assert_eq!(display.segment(&snapshot, header_rows.end).line, 3);
    let last_row = display.segment(&snapshot, header_rows.end - 1);
    place(&mut store, view, last_row.bytes.start, last_row.bytes.start);
    move_selection_displayed(&mut store, view, DOWN, false, &display).unwrap();
    assert_eq!(
        caret(&store, view),
        (line_start(&snapshot, 3), line_start(&snapshot, 3))
    );
    move_selection_displayed(&mut store, view, UP, false, &display).unwrap();
    assert_eq!(
        caret(&store, view),
        (last_row.bytes.start, last_row.bytes.start)
    );
    for byte in line_start(&snapshot, 1)..=line_end(&snapshot, 2) {
        assert_eq!(display.row_of_byte(&snapshot, byte), header_rows.end - 1);
    }
}
