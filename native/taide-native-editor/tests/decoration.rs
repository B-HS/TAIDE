use std::borrow::Cow;
use std::ops::Range;
use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, LARGE_FILE_BYTES, OpenedFile};
use taide_native_editor::change_journal::{ChangeSet, ChangesSince, MAX_JOURNAL_ENTRIES};
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, LaneMark, Stickiness, Underline,
    UnderlineKind,
};
use taide_native_editor::document::{DocumentId, Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};

const DOCUMENT_COUNT: usize = 4;
const VIEW_COUNT: usize = 8;
const HISTORY_COUNT: usize = 8;
const SYNTHETIC_PATH: &str = "/synthetic/main.rs";
const LETTERS: &str = "abcdef";
const MARKED: Range<usize> = 2..4;
const WIDE: Range<usize> = 0..50;
const WARNING: [u8; 4] = [204, 167, 0, 255];
const ADDED: [u8; 4] = [46, 160, 67, 255];
const CONFLICT: [u8; 4] = [64, 200, 174, 51];
const FIND_MATCH: [u8; 4] = [234, 92, 0, 85];
const BRACKET: [u8; 4] = [255, 215, 0, 255];
const DIAGNOSTICS_Z_ORDER: u8 = 20;
const STICKINESSES: [Stickiness; 4] = [
    Stickiness::AlwaysGrowsWhenTypingAtEdges,
    Stickiness::NeverGrowsWhenTypingAtEdges,
    Stickiness::GrowsOnlyWhenTypingBefore,
    Stickiness::GrowsOnlyWhenTypingAfter,
];

#[test]
fn 가장자리_입력은_stickiness에_따라_범위를_늘리거나_민다() {
    let at_start = [2..5, 3..5, 2..5, 3..5];
    let at_end = [2..5, 2..4, 2..4, 2..5];
    for (index, stickiness) in STICKINESSES.into_iter().enumerate() {
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(2..2, "X")]),
            at_start[index],
            "{stickiness:?} start edge"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(4..4, "X")]),
            at_end[index],
            "{stickiness:?} end edge"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(3..3, "X")]),
            2..5,
            "{stickiness:?} inside"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(1..1, "X")]),
            3..5,
            "{stickiness:?} before"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(5..5, "X")]),
            MARKED,
            "{stickiness:?} after"
        );
    }
}

#[test]
fn 빈_범위에서의_입력은_stickiness에_따라_글자를_품거나_앞뒤에_남는다() {
    let collapsed = [3..4, 4..4, 3..3, 4..4];
    for (index, stickiness) in STICKINESSES.into_iter().enumerate() {
        assert_eq!(
            moved(LETTERS, 3..3, stickiness, vec![(3..3, "X")]),
            collapsed[index],
            "{stickiness:?}"
        );
    }
}

#[test]
fn 삭제는_겹친_부분만_줄이고_범위를_덮으면_삭제_시작의_빈_범위로_남긴다() {
    for stickiness in STICKINESSES {
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(1..3, "")]),
            1..2,
            "{stickiness:?} head"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(3..5, "")]),
            2..3,
            "{stickiness:?} tail"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(1..5, "")]),
            1..1,
            "{stickiness:?} covered"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(2..4, "")]),
            2..2,
            "{stickiness:?} exact"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(0..1, "")]),
            1..3,
            "{stickiness:?} before"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(4..6, "")]),
            MARKED,
            "{stickiness:?} after"
        );
    }
}

#[test]
fn 바꾸기는_공통_길이_안의_경계를_제자리에_두고_그_뒤의_경계는_새_글자_끝으로_옮긴다() {
    let exact_longer = [2..5, 2..4, 2..4, 2..5];
    for (index, stickiness) in STICKINESSES.into_iter().enumerate() {
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(1..3, "XY")]),
            MARKED,
            "{stickiness:?} same length"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(1..3, "X")]),
            2..3,
            "{stickiness:?} shorter"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(1..3, "XYZ")]),
            2..5,
            "{stickiness:?} longer"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(2..4, "XYZ")]),
            exact_longer[index],
            "{stickiness:?} exact longer"
        );
        assert_eq!(
            moved(LETTERS, MARKED, stickiness, vec![(0..6, "X")]),
            1..1,
            "{stickiness:?} whole document"
        );
    }
}

#[test]
fn 한_transaction의_여러_편집은_각_범위가_같은_글자를_가리키게_옮긴다() {
    let before = "let 값 = 1;\nlet b = 2;\nend";
    let mut store = store();
    let document = open(&mut store, before);
    let value = before.find('값').unwrap();
    let second = before.find('b').unwrap();
    let last = before.find("end").unwrap();
    let mut layer = DecorationLayer::new(
        0,
        DIAGNOSTICS_Z_ORDER,
        vec![
            squiggle(
                value..value + '값'.len_utf8(),
                Stickiness::NeverGrowsWhenTypingAtEdges,
            ),
            squiggle(second..second + 1, Stickiness::NeverGrowsWhenTypingAtEdges),
            line_background(last..last),
        ],
    );
    apply(
        &mut store,
        document,
        vec![
            (0..0, "// 머리\n"),
            (value..value, "mut "),
            (second + 1..second + 1, "eta"),
            (last - 1..last - 1, "\n\n"),
        ],
        0,
    );
    let after = text(&store, document);
    assert_eq!(after, "// 머리\nlet mut 값 = 1;\nlet beta = 2;\n\n\nend");
    let sets = tracked(&store, document, 0);
    assert_eq!(sets[0].spans.len(), 4);
    assert!(layer.apply(&sets[0]));
    assert_eq!(layer.revision(), 1);
    let covered: Vec<&str> = layer
        .items()
        .iter()
        .map(|item| &after[item.bytes.clone()])
        .collect();
    assert_eq!(covered, ["값", "b", ""]);
    assert_eq!(layer.items()[2].bytes.start, after.find("end").unwrap());
}

#[test]
fn undo와_redo의_span으로도_범위가_글자를_따라간다() {
    let before = "let a = 1;\nlet b = 2;\n";
    let mut store = store();
    let document = open(&mut store, before);
    apply(&mut store, document, vec![(4..4, "mut ")], 0);
    let edited = text(&store, document);
    let name = edited.find('a').unwrap();
    let other = edited.find('b').unwrap();
    let mut layer = DecorationLayer::new(
        1,
        DIAGNOSTICS_Z_ORDER,
        vec![
            squiggle(name..name + 1, Stickiness::NeverGrowsWhenTypingAtEdges),
            squiggle(other..other + 1, Stickiness::NeverGrowsWhenTypingAtEdges),
        ],
    );
    assert!(store.undo(document).unwrap());
    for set in tracked(&store, document, 1) {
        assert!(layer.apply(&set));
    }
    assert_eq!(layer.revision(), 2);
    assert_eq!(ranges(&layer), [4..5, 15..16]);
    assert!(store.redo(document).unwrap());
    for set in tracked(&store, document, 2) {
        assert!(layer.apply(&set));
    }
    assert_eq!(layer.revision(), 3);
    assert_eq!(ranges(&layer), [name..name + 1, other..other + 1]);
}

#[test]
fn 저널을_따라가는_층은_변경이_없으면_빌린_그대로이고_뒤처지거나_어긋나면_무효다() {
    let mut store = store();
    let document = open(&mut store, LETTERS);
    let layer = DecorationLayer::new(
        0,
        DIAGNOSTICS_Z_ORDER,
        vec![squiggle(MARKED, Stickiness::NeverGrowsWhenTypingAtEdges)],
    );
    let current = layer
        .tracking(store.changes_since(document, 0).unwrap())
        .unwrap();
    assert!(matches!(current, Cow::Borrowed(_)));
    apply(&mut store, document, vec![(0..0, "12")], 0);
    apply(&mut store, document, vec![(0..1, "")], 1);
    let followed = layer
        .tracking(store.changes_since(document, 0).unwrap())
        .unwrap();
    assert!(matches!(followed, Cow::Owned(_)));
    assert_eq!(followed.revision(), 2);
    assert_eq!(followed.items().len(), 1);
    assert_eq!(followed.items()[0].bytes, 3..5);
    assert_eq!(layer.revision(), 0);
    assert_eq!(ranges(&layer), [MARKED]);

    let mut stale = layer.clone();
    let sets = tracked(&store, document, 0);
    assert!(!stale.apply(&sets[1]));
    assert_eq!(stale, layer);

    for group in 0..MAX_JOURNAL_ENTRIES {
        apply(
            &mut store,
            document,
            vec![(0..0, "x")],
            u64::try_from(group).unwrap() + 2,
        );
    }
    assert!(matches!(
        store.changes_since(document, 0).unwrap(),
        ChangesSince::Lagged
    ));
    assert!(
        layer
            .tracking(store.changes_since(document, 0).unwrap())
            .is_none()
    );
    let future = DecorationLayer::new(u64::MAX, DIAGNOSTICS_Z_ORDER, Vec::new());
    assert!(
        future
            .tracking(store.changes_since(document, u64::MAX).unwrap())
            .is_none()
    );
}

#[test]
fn 층은_시작_바이트_순으로_정렬되고_범위와_닿는_장식만_돌려준다() {
    let bar = DecorationKind::Lane {
        mark: LaneMark::Bar,
        color: ADDED,
    };
    let triangle = DecorationKind::Lane {
        mark: LaneMark::DeletedTriangle,
        color: ADDED,
    };
    let highlighted = DecorationKind::Inline(InlineStyle {
        foreground: Some(BRACKET),
        background: Some(FIND_MATCH),
        underline: Some(Underline {
            kind: UnderlineKind::Straight,
            color: BRACKET,
        }),
    });
    let decoration = |bytes: Range<usize>, kind: DecorationKind| Decoration {
        bytes,
        kind,
        stickiness: Stickiness::default(),
    };
    let mut layer = DecorationLayer::new(
        7,
        DIAGNOSTICS_Z_ORDER,
        vec![
            decoration(30..40, bar),
            decoration(WIDE, highlighted),
            decoration(12..12, triangle),
            decoration(5..8, bar),
        ],
    );
    assert_eq!(layer.revision(), 7);
    assert_eq!(layer.z_order(), DIAGNOSTICS_Z_ORDER);
    assert_eq!(ranges(&layer), [WIDE, 5..8, 12..12, 30..40]);
    assert_eq!(
        Stickiness::default(),
        Stickiness::AlwaysGrowsWhenTypingAtEdges
    );
    let touching = |bytes: Range<usize>| -> Vec<Range<usize>> {
        layer
            .intersecting(bytes)
            .map(|item| item.bytes.clone())
            .collect()
    };
    assert_eq!(touching(9..11), [WIDE]);
    assert_eq!(touching(8..12), [WIDE, 5..8, 12..12]);
    assert_eq!(touching(41..60), [WIDE]);
    assert_eq!(touching(40..40), [WIDE, 30..40]);
    assert!(touching(51..60).is_empty());

    let mut store = store();
    let document = open(&mut store, &"x".repeat(60));
    for _ in 0..layer.revision() {
        let revision = store.documents().snapshot(document).unwrap().revision;
        apply(&mut store, document, vec![(59..60, "x")], revision);
    }
    apply(&mut store, document, vec![(5..5, "12345")], 7);
    for set in tracked(&store, document, 7) {
        assert!(layer.apply(&set));
    }
    assert_eq!(ranges(&layer), [0..55, 5..13, 17..17, 35..45]);
    assert_eq!(layer.items()[0].kind, highlighted);
    assert_eq!(layer.items()[1].kind, bar);
    assert_eq!(layer.items()[2].kind, triangle);
}

fn squiggle(bytes: Range<usize>, stickiness: Stickiness) -> Decoration {
    Decoration {
        bytes,
        kind: DecorationKind::Inline(InlineStyle {
            underline: Some(Underline {
                kind: UnderlineKind::Squiggly,
                color: WARNING,
            }),
            ..InlineStyle::default()
        }),
        stickiness,
    }
}

fn line_background(bytes: Range<usize>) -> Decoration {
    Decoration {
        bytes,
        kind: DecorationKind::LineBackground(CONFLICT),
        stickiness: Stickiness::default(),
    }
}

fn ranges(layer: &DecorationLayer) -> Vec<Range<usize>> {
    layer
        .items()
        .iter()
        .map(|item| item.bytes.clone())
        .collect()
}

fn moved(
    content: &str,
    bytes: Range<usize>,
    stickiness: Stickiness,
    edits: Vec<(Range<usize>, &str)>,
) -> Range<usize> {
    let mut store = store();
    let document = open(&mut store, content);
    let mut layer = DecorationLayer::new(0, DIAGNOSTICS_Z_ORDER, vec![squiggle(bytes, stickiness)]);
    apply(&mut store, document, edits, 0);
    for set in tracked(&store, document, 0) {
        assert!(layer.apply(&set));
    }
    layer.items()[0].bytes.clone()
}

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: LARGE_FILE_BYTES as usize,
    })
    .unwrap()
}

fn open(store: &mut EditorStore, content: &str) -> DocumentId {
    store
        .open_file(
            PathBuf::from(SYNTHETIC_PATH),
            OpenedFile {
                path: SYNTHETIC_PATH.into(),
                content: content.into(),
                language_id: "rust".into(),
                byte_size: content.len().try_into().unwrap(),
                line_count: content.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap()
}

fn text(store: &EditorStore, document: DocumentId) -> String {
    store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string()
}

fn apply(
    store: &mut EditorStore,
    document: DocumentId,
    edits: Vec<(Range<usize>, &str)>,
    group: u64,
) -> u64 {
    let revision = store.documents().snapshot(document).unwrap().revision;
    store
        .apply_separate(
            document,
            Transaction {
                revision,
                edits: edits
                    .into_iter()
                    .map(|(bytes, text)| Edit {
                        bytes,
                        text: text.into(),
                    })
                    .collect(),
                group: UndoGroup(group),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap()
}

fn tracked(store: &EditorStore, document: DocumentId, revision: u64) -> Vec<ChangeSet> {
    match store.changes_since(document, revision).unwrap() {
        ChangesSince::Tracked(changes) => changes.cloned().collect(),
        ChangesSince::Lagged => panic!("journal lagged behind revision {revision}"),
    }
}
