use std::fmt::Debug;
use std::ops::Range;
use std::str::FromStr;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_native_editor::document::{DocumentId, Edit, UndoGroup};
use taide_native_editor::folding::FoldRegion;
use taide_native_editor::sticky_model::{
    StickyCandidate, StickyModel, StickyScope, StickyViewport,
};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};

const SOURCE_CASES: usize = 357;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024 * 1024;
const LINE_HEIGHT: f32 = 20.0;
const DEEP_SCOPES: usize = 4096;
const DEEP_QUERY_LINE: usize = DEEP_SCOPES / 2;
const FIELDS: usize = 13;

fn fixture(lines: usize) -> (EditorStore, DocumentId) {
    let content = std::iter::repeat_n("line", lines)
        .collect::<Vec<_>>()
        .join("\n");
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/sticky.txt".into(),
            OpenedFile {
                path: "/synthetic/sticky.txt".into(),
                byte_size: content.len().try_into().unwrap(),
                content,
                language_id: "plaintext".into(),
                line_count: lines.try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    (store, document)
}

fn numbers<T: FromStr>(text: &str) -> Vec<T>
where
    T::Err: Debug,
{
    text.split(',')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap())
        .collect()
}

#[test]
fn 원본_고정줄_357표본의_후보와_중첩_최대_밀림_접기_줄나눔이_일치한다() {
    let mut source = include_str!("fixtures/sticky-scroll-reference.txt").lines();
    assert_eq!(source.next(), Some("0.56.0\t20\t357"));
    let mut count = 0;
    for record in source {
        let fields = record.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), FIELDS);
        let mut fields = fields.into_iter();
        let line_count = fields.next().unwrap().parse().unwrap();
        let height = fields.next().unwrap().parse().unwrap();
        let scroll_top = fields.next().unwrap().parse().unwrap();
        let first = fields.next().unwrap().parse().unwrap();
        let end = fields.next().unwrap().parse().unwrap();
        let scopes = numbers::<usize>(fields.next().unwrap())
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[start_line, end_line]| StickyScope {
                start_line: *start_line,
                end_line: *end_line,
            })
            .collect::<Vec<_>>();
        let hidden = numbers::<usize>(fields.next().unwrap())
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[start, end]| *start..*end)
            .collect::<Vec<Range<usize>>>();
        let tops = numbers::<f32>(fields.next().unwrap());
        let bottoms = numbers::<f32>(fields.next().unwrap());
        let candidates = numbers::<usize>(fields.next().unwrap())
            .as_chunks::<3>()
            .0
            .iter()
            .map(|[start_line, end_line, level]| StickyCandidate {
                scope: StickyScope {
                    start_line: *start_line,
                    end_line: *end_line,
                },
                level: *level,
            })
            .collect::<Vec<_>>();
        let starts = numbers::<usize>(fields.next().unwrap());
        let ends = numbers::<usize>(fields.next().unwrap());
        let offset: f32 = fields.next().unwrap().parse().unwrap();
        let (store, document) = fixture(line_count);
        let model = StickyModel::new(&store.documents().snapshot(document).unwrap(), &scopes);
        assert_eq!(
            model.candidates(first..end, &hidden),
            candidates,
            "case {count}"
        );
        let viewport = StickyViewport {
            visible_lines: first..end,
            height,
            scroll_top,
            line_height: LINE_HEIGHT,
        };
        let layout = model.layout(&viewport, &hidden, |line| tops[line], |line| bottoms[line]);
        let actual = layout
            .scopes
            .iter()
            .map(|scope| (scope.start_line, scope.end_line))
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            starts.into_iter().zip(ends).collect::<Vec<_>>(),
            "case {count}"
        );
        assert_eq!(layout.last_relative_position, offset, "case {count}");
        count += 1;
    }
    assert_eq!(count, SOURCE_CASES);
}

#[test]
fn 문서_버전을_확인하고_접기_모델의_종료_줄을_변환한다() {
    let (mut store, document) = fixture(10);
    let snapshot = store.documents().snapshot(document).unwrap();
    let model = StickyModel::from_folds(
        &snapshot,
        &[FoldRegion {
            start_line: 1,
            end_line: 7,
        }],
    );
    assert!(model.describes(&snapshot));
    let mut other_language = snapshot.clone();
    other_language.metadata.language_id = "different-language".into();
    assert!(!model.describes(&other_language));
    assert_eq!(
        model.candidates(3..5, &[]),
        vec![StickyCandidate {
            scope: StickyScope {
                start_line: 1,
                end_line: 8
            },
            level: 0,
        }]
    );
    let transaction = Transaction {
        revision: snapshot.revision,
        edits: vec![Edit {
            bytes: 0..0,
            text: "x".into(),
        }],
        group: UndoGroup(1),
        origin: None,
        selection_after: None,
    };
    store.apply(document, transaction).unwrap();
    assert!(!model.describes(&store.documents().snapshot(document).unwrap()));
    assert!(model.describes(&snapshot));
}

#[test]
fn 작은_화면과_잘못된_크기는_본문을_고정_줄로_덮지_않는다() {
    let (store, document) = fixture(10);
    let model = StickyModel::new(
        &store.documents().snapshot(document).unwrap(),
        &[StickyScope {
            start_line: 0,
            end_line: 8,
        }],
    );
    for height in [1.0, 10.0, f32::NAN, f32::INFINITY] {
        let layout = model.layout(
            &StickyViewport {
                visible_lines: 2..3,
                scroll_top: LINE_HEIGHT * 2.0,
                height,
                line_height: LINE_HEIGHT,
            },
            &[],
            |line| line as f32 * LINE_HEIGHT,
            |line| (line + 1) as f32 * LINE_HEIGHT,
        );
        assert!(layout.scopes.is_empty());
        assert_eq!(layout.height(LINE_HEIGHT), 0.0);
    }
}

#[test]
fn 깊은_고정_줄은_재귀_없이_최대_줄만_표시하고_잘못된_범위를_제외한다() {
    let (store, document) = fixture(DEEP_SCOPES + 2);
    let mut scopes = (0..DEEP_SCOPES)
        .map(|start_line| StickyScope {
            start_line,
            end_line: DEEP_SCOPES + 1,
        })
        .collect::<Vec<_>>();
    scopes.extend([
        StickyScope {
            start_line: usize::MAX,
            end_line: usize::MAX,
        },
        StickyScope {
            start_line: 3,
            end_line: 2,
        },
    ]);
    let model = StickyModel::new(&store.documents().snapshot(document).unwrap(), &scopes);
    let layout = model.layout(
        &StickyViewport {
            visible_lines: DEEP_QUERY_LINE..DEEP_QUERY_LINE + 1,
            scroll_top: DEEP_QUERY_LINE as f32 * LINE_HEIGHT,
            height: LINE_HEIGHT * 40.0,
            line_height: LINE_HEIGHT,
        },
        &[],
        |line| line as f32 * LINE_HEIGHT,
        |line| (line + 1) as f32 * LINE_HEIGHT,
    );
    assert_eq!(layout.scopes.len(), 5);
    assert_eq!(layout.scopes.last().unwrap().start_line, 4);
}
