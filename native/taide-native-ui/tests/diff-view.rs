#![cfg(all(feature = "native-host", feature = "inspection"))]

use egui::{
    Color32, Context, Event, FontId, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Shape, pos2,
};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, VirtualKey, VirtualSide};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ScrollPosition, ViewId, ViewKey};
use taide_native_ui::diff_view::{self, Colors, Hunk, Output, Request, Tokens};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorPresentation, EditorTokens, ExtraZone, NativeEditor,
};

const ORIGINAL: &str = "one\ntwo\nthree\nfour\nfive\n";
const MODIFIED: &str = "zero\none\nTWO\nthree\nfive\n";
const HUNKS: [Hunk; 3] = [
    Hunk {
        old_start: 0,
        old_lines: 0,
        new_start: 1,
        new_lines: 1,
    },
    Hunk {
        old_start: 2,
        old_lines: 1,
        new_start: 3,
        new_lines: 1,
    },
    Hunk {
        old_start: 4,
        old_lines: 1,
        new_start: 4,
        new_lines: 0,
    },
];
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const FRAME_STEP: f64 = 0.1;
const BYTE_LIMIT: usize = 4096;
const WIDE: egui::Vec2 = egui::vec2(1200.0, 600.0);
const NARROW: egui::Vec2 = egui::vec2(900.0, 600.0);
const SCROLL_TOP: f32 = 40.0;
const SASH_TARGET_X: f32 = 300.0;
const LANE_INSET: f32 = 2.0;
const HALF: f32 = 0.5;
const COLORS: Colors = Colors {
    inserted_line: Color32::from_rgb(1, 200, 1),
    removed_line: Color32::from_rgb(200, 1, 1),
    diagonal_fill: Color32::from_rgb(9, 9, 200),
    border: Color32::from_rgb(7, 7, 7),
    stage: Color32::from_rgb(150, 1, 150),
    foreground: Color32::WHITE,
    unchanged_code: Color32::from_rgb(3, 3, 90),
    unchanged_region: Color32::from_rgb(4, 90, 4),
    unchanged_region_foreground: Color32::from_rgb(250, 250, 5),
};
const LONG_LINES: usize = 40;
const LONG_CHANGED_LINE: usize = 20;

struct NoTokens;

impl Tokens for NoTokens {
    fn tokens(&mut self, _: &EditorStore, _: DocumentId) -> Option<EditorTokens<'_>> {
        None
    }
}

struct Fixture {
    context: Context,
    store: EditorStore,
    original: ViewId,
    modified: ViewId,
    editor: NativeEditor,
    presentation: EditorPresentation,
    time: f64,
    screen: egui::Vec2,
    side_by_side: bool,
    hunks: Vec<Hunk>,
    hide_unchanged: bool,
}

impl Fixture {
    fn new(screen: egui::Vec2) -> Self {
        Self::with(screen, ORIGINAL, MODIFIED, HUNKS.to_vec())
    }

    fn with(
        screen: egui::Vec2,
        original_text: &str,
        modified_text: &str,
        hunks: Vec<Hunk>,
    ) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 2,
            max_views: 2,
            max_undo_groups: 4,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let owner = TabId::new();
        let mut attach = |side, text: &str| {
            let document = store
                .open_virtual(
                    VirtualKey {
                        owner: owner.clone(),
                        side,
                    },
                    text,
                    "plaintext".into(),
                    true,
                )
                .unwrap();
            store
                .attach_view(
                    ViewKey {
                        window: "diff-view-fixture".into(),
                        pane: PaneId::new(),
                        tab: TabId::new(),
                    },
                    document,
                )
                .unwrap()
        };
        let original = attach(VirtualSide::Original, original_text);
        let modified = attach(VirtualSide::Modified, modified_text);
        Self {
            context: Context::default(),
            store,
            original,
            modified,
            editor: NativeEditor {
                appearance: EditorAppearance {
                    font: FontId::monospace(FONT_SIZE),
                    line_height: LINE_HEIGHT,
                    horizontal_padding: 0.0,
                    background: Color32::BLACK,
                    foreground: Color32::WHITE,
                    muted: Color32::GRAY,
                    selection: Color32::BLUE,
                    cursor: Color32::WHITE,
                    current_line: Color32::TRANSPARENT,
                    line_numbers: false,
                    indent: "    ".into(),
                },
            },
            presentation: EditorPresentation::default(),
            time: 0.0,
            screen,
            side_by_side: true,
            hunks,
            hide_unchanged: false,
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> (Output, FullOutput) {
        self.time += FRAME_STEP;
        let mut shown = None;
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen)),
                time: Some(self.time),
                events,
                focused: true,
                ..Default::default()
            },
            |ui| {
                shown = Some(
                    diff_view::show(
                        ui,
                        &mut self.store,
                        &self.editor,
                        Request {
                            original: self.original,
                            modified: self.modified,
                            hunks: &self.hunks,
                            side_by_side: self.side_by_side,
                            stage_lanes: true,
                            hide_unchanged: self.hide_unchanged,
                            presentation: &self.presentation,
                            colors: COLORS,
                        },
                        &mut NoTokens,
                    )
                    .unwrap(),
                );
            },
        );
        output.textures_delta.clear();
        (shown.unwrap(), output)
    }

    fn press(&mut self, position: Pos2) -> Output {
        let button = |pressed| Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let (shown, _) = self.frame(vec![Event::PointerMoved(position), button(true)]);
        self.frame(vec![button(false)]);
        shown
    }
}

fn filled_tops(output: &FullOutput, color: Color32) -> Vec<f32> {
    let mut tops = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Rect(rect) if rect.fill == color => Some(rect.rect.top()),
            _ => None,
        })
        .collect::<Vec<_>>();
    tops.sort_by(f32::total_cmp);
    tops.dedup();
    tops
}

fn painted_texts(output: &FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn 채움_띠는_짧은_쪽_hunk_뒤에_줄_수_차만큼_놓이고_맨_위_삽입은_첫_줄_위다() {
    let (original, modified) = diff_view::fillers(&HUNKS, LINE_HEIGHT);
    assert_eq!(
        original,
        vec![ExtraZone {
            after_line: None,
            height: LINE_HEIGHT
        }]
    );
    assert_eq!(
        modified,
        vec![ExtraZone {
            after_line: Some(3),
            height: LINE_HEIGHT
        }]
    );
    let inline = diff_view::inline_zones(&HUNKS, LINE_HEIGHT);
    assert_eq!(
        inline,
        vec![
            (
                ExtraZone {
                    after_line: Some(1),
                    height: LINE_HEIGHT
                },
                1..2
            ),
            (
                ExtraZone {
                    after_line: Some(3),
                    height: LINE_HEIGHT
                },
                3..4
            ),
        ]
    );
    assert_eq!(diff_view::stage_ranges(&HUNKS), vec![1..=1, 3..=3, 4..=4]);
    assert!(!diff_view::is_side_by_side(
        true,
        diff_view::INLINE_BREAKPOINT
    ));
    assert!(diff_view::is_side_by_side(
        true,
        diff_view::INLINE_BREAKPOINT + 1.0
    ));
    assert!(!diff_view::is_side_by_side(false, WIDE.x));
}

#[test]
fn 좌우_보기는_채움_띠로_같은_줄을_같은_높이에_두고_줄_배경과_빗금을_그린다() {
    let mut fixture = Fixture::new(WIDE);
    fixture.frame(Vec::new());
    let (shown, painted) = fixture.frame(Vec::new());
    assert!(shown.is_side_by_side);
    let original = shown.original.as_ref().expect("원본 편집기");
    let top = original.geometry.rect.top();
    assert_eq!(
        original.extra_zones,
        vec![Some(Rect::from_min_max(
            pos2(original.geometry.content_rect.left(), top),
            pos2(original.geometry.rect.right(), top + LINE_HEIGHT)
        ))]
    );
    let modified_zone = shown.modified.extra_zones[0].expect("수정 쪽 채움 띠");
    assert_eq!(modified_zone.top(), top + 4.0 * LINE_HEIGHT);
    assert_eq!(modified_zone.height(), LINE_HEIGHT);
    assert_eq!(
        filled_tops(&painted, COLORS.removed_line),
        vec![top + 2.0 * LINE_HEIGHT, top + 4.0 * LINE_HEIGHT]
    );
    assert_eq!(
        filled_tops(&painted, COLORS.inserted_line),
        vec![top, top + 2.0 * LINE_HEIGHT]
    );
    assert!(painted.shapes.iter().any(|shape| matches!(
        &shape.shape,
        Shape::LineSegment { stroke, .. } if stroke.color == COLORS.diagonal_fill
    )));
    let five = painted
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.text() == "five" => Some(text.pos.y),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(five.len(), 2, "두 편집기 모두 five 를 그려야 합니다");
    assert_eq!(
        five[0], five[1],
        "채움 띠 뒤 같은 줄은 같은 높이여야 합니다"
    );
}

#[test]
fn 폭_900_이하는_수정_쪽만_그리고_삭제_줄을_띠_안에_보인다() {
    let mut fixture = Fixture::new(NARROW);
    fixture.frame(Vec::new());
    let (shown, painted) = fixture.frame(Vec::new());
    assert!(!shown.is_side_by_side);
    assert!(shown.original.is_none());
    let top = shown.modified.geometry.rect.top();
    let zones = shown
        .modified
        .extra_zones
        .iter()
        .map(|zone| zone.expect("삭제 줄 띠").y_range())
        .collect::<Vec<_>>();
    assert_eq!(zones[0].min, top + 2.0 * LINE_HEIGHT);
    assert_eq!(zones[1].min, top + 5.0 * LINE_HEIGHT);
    let texts = painted_texts(&painted);
    assert!(texts.contains(&"two".to_owned()));
    assert!(texts.contains(&"four".to_owned()));
    assert!(texts.contains(&"TWO".to_owned()));

    fixture.screen = WIDE;
    fixture.side_by_side = false;
    let (shown, _) = fixture.frame(Vec::new());
    assert!(
        !shown.is_side_by_side,
        "요청이 inline 이면 넓어도 inline 입니다"
    );
}

#[test]
fn 한쪽_스크롤은_다음_프레임에_다른_쪽에_같은_값으로_반영된다() {
    let mut fixture = Fixture::new(egui::vec2(WIDE.x, 3.0 * LINE_HEIGHT));
    fixture.frame(Vec::new());
    let view = fixture.store.views().get(fixture.modified).unwrap().clone();
    fixture
        .store
        .set_view_state(
            fixture.modified,
            view.selection,
            ScrollPosition {
                x: 0.0,
                y: SCROLL_TOP,
            },
            view.folds,
        )
        .unwrap();
    fixture.frame(Vec::new());
    let (shown, _) = fixture.frame(Vec::new());
    assert_eq!(shown.modified.geometry.scroll.y, SCROLL_TOP);
    assert_eq!(
        shown.original.as_ref().unwrap().geometry.scroll.y,
        SCROLL_TOP
    );
}

#[test]
fn 가운데_분할선을_끌면_원본_폭이_바뀐다() {
    let mut fixture = Fixture::new(WIDE);
    let (shown, _) = fixture.frame(Vec::new());
    let sash = shown.sash.expect("분할선");
    assert_eq!(
        shown.original.as_ref().unwrap().geometry.rect.width(),
        WIDE.x * diff_view::DEFAULT_SPLIT
    );
    let start = sash.center();
    let target = pos2(SASH_TARGET_X, start.y);
    fixture.frame(vec![Event::PointerMoved(start)]);
    fixture.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    fixture.frame(vec![Event::PointerMoved(target)]);
    fixture.frame(vec![
        Event::PointerMoved(target),
        Event::PointerButton {
            pos: target,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]);
    let (shown, _) = fixture.frame(Vec::new());
    assert_eq!(
        shown.original.as_ref().unwrap().geometry.rect.width(),
        SASH_TARGET_X
    );
}

#[test]
fn 수정_쪽_lane_칸을_누르면_그_줄의_hunk_범위를_돌려주고_범위_밖은_무시한다() {
    let mut fixture = Fixture::new(WIDE);
    let (shown, painted) = fixture.frame(Vec::new());
    assert_eq!(
        filled_tops(&painted, COLORS.stage).len(),
        3,
        "stage 막대는 hunk 마다 하나씩입니다"
    );
    let rect = shown.modified.geometry.rect;
    let at_line = |line: f32| {
        pos2(
            rect.left() + LANE_INSET,
            rect.top() + (line + HALF) * LINE_HEIGHT,
        )
    };
    assert_eq!(fixture.press(at_line(2.0)).lane_press, Some(3..=3));
    assert_eq!(fixture.press(at_line(5.0)).lane_press, None);
    assert_eq!(
        fixture
            .press(pos2(
                rect.center().x,
                rect.top() + (2.0 + HALF) * LINE_HEIGHT
            ))
            .lane_press,
        None
    );
}

fn long_text(changed: Option<&str>) -> String {
    (1..=LONG_LINES)
        .map(|line| match changed {
            Some(text) if line == LONG_CHANGED_LINE => format!("{text}\n"),
            _ => format!("line {line:02}\n"),
        })
        .collect()
}

fn count(texts: &[String], wanted: &str) -> usize {
    texts.iter().filter(|text| *text == wanted).count()
}

#[test]
fn 미변경_영역은_문맥_3줄을_남기고_최소_3줄부터_접고_펼침은_20줄씩이다() {
    let changed = Hunk {
        old_start: 20,
        old_lines: 1,
        new_start: 20,
        new_lines: 1,
    };
    let regions = diff_view::unchanged_regions(&[changed], 41, 41);
    assert_eq!(
        regions,
        vec![
            diff_view::UnchangedRegion {
                original_start: 1,
                modified_start: 1,
                line_count: 16
            },
            diff_view::UnchangedRegion {
                original_start: 24,
                modified_start: 24,
                line_count: 18
            },
        ]
    );
    let between = |second: u32| {
        diff_view::unchanged_regions(
            &[
                Hunk {
                    old_start: 1,
                    old_lines: 1,
                    new_start: 1,
                    new_lines: 1,
                },
                Hunk {
                    old_start: second,
                    old_lines: 1,
                    new_start: second,
                    new_lines: 1,
                },
            ],
            second + 1,
            second + 1,
        )
    };
    assert!(between(10).is_empty(), "사이 8줄은 접지 않습니다");
    assert_eq!(
        between(11),
        vec![diff_view::UnchangedRegion {
            original_start: 5,
            modified_start: 5,
            line_count: 3
        }]
    );
    let inserted = diff_view::unchanged_regions(
        &[Hunk {
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: 2,
        }],
        10,
        12,
    );
    assert_eq!(
        inserted,
        vec![diff_view::UnchangedRegion {
            original_start: 4,
            modified_start: 6,
            line_count: 7
        }],
        "맨 위 삽입 뒤 영역은 양쪽 시작이 다르고 끝까지 접습니다"
    );

    let start = regions[0];
    let hidden = start.hidden(diff_view::Reveal::default());
    assert_eq!(
        (hidden.original_first, hidden.count),
        (2, 15),
        "첫 줄은 편집기가 숨기지 않으므로 남깁니다"
    );
    let above = start.show_more_above(diff_view::Reveal::default());
    assert_eq!(above.top, 16, "펼침은 영역 줄 수를 넘지 않습니다");
    assert_eq!(start.hidden(above).count, 0);
    let end = regions[1];
    let below = end.show_more_below(diff_view::Reveal::default());
    assert_eq!(
        (below.bottom, end.hidden(below).count),
        (18, 0),
        "18줄 영역의 아래 20줄 펼침은 18줄에서 멈춥니다"
    );
    let partial = diff_view::UnchangedRegion {
        original_start: 4,
        modified_start: 4,
        line_count: 30,
    };
    let below = partial.show_more_below(diff_view::Reveal::default());
    assert_eq!(
        (below.bottom, partial.hidden(below).count),
        (diff_view::UNCHANGED_REVEAL_LINES, 10)
    );
    let all = end.show_all(diff_view::Reveal { top: 2, bottom: 0 });
    assert_eq!(all.bottom, 16);
}

#[test]
fn 숨김_설정은_양쪽에_막대를_그리고_위_가장자리는_20줄을_아이콘은_전부를_펼친다() {
    const TALL: egui::Vec2 = egui::vec2(1200.0, 1800.0);
    let hunks = vec![Hunk {
        old_start: LONG_CHANGED_LINE as u32,
        old_lines: 1,
        new_start: LONG_CHANGED_LINE as u32,
        new_lines: 1,
    }];
    let mut fixture = Fixture::with(TALL, &long_text(None), &long_text(Some("LINE 20")), hunks);
    fixture.frame(Vec::new());
    let (_, painted) = fixture.frame(Vec::new());
    assert!(
        !painted_texts(&painted)
            .iter()
            .any(|text| text.ends_with("hidden lines")),
        "설정이 꺼져 있으면 접지 않습니다"
    );

    fixture.hide_unchanged = true;
    fixture.frame(Vec::new());
    let (shown, painted) = fixture.frame(Vec::new());
    let texts = painted_texts(&painted);
    assert_eq!(count(&texts, "15 hidden lines"), 2, "양쪽 첫 막대");
    assert_eq!(count(&texts, "18 hidden lines"), 2, "양쪽 끝 막대");
    assert_eq!(count(&texts, "line 01"), 2);
    assert_eq!(count(&texts, "line 05"), 0, "숨긴 줄은 그리지 않습니다");
    assert_eq!(count(&texts, "line 17"), 2, "변경 앞 문맥 3줄");
    assert_eq!(count(&texts, "line 23"), 2, "변경 뒤 문맥 3줄");
    assert_eq!(count(&texts, "line 30"), 0);
    assert!(!filled_tops(&painted, COLORS.unchanged_region).is_empty());
    assert!(!filled_tops(&painted, COLORS.unchanged_code).is_empty());
    let bars = shown
        .modified
        .extra_zones
        .iter()
        .map(|zone| zone.expect("막대 zone"))
        .collect::<Vec<_>>();
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].height(), diff_view::UNCHANGED_BAR_HEIGHT);
    let top = shown.modified.geometry.rect.top();
    assert_eq!(bars[0].top(), top + LINE_HEIGHT, "첫 줄 바로 뒤");

    fixture.press(pos2(bars[1].center().x, bars[1].top() - 1.0));
    let (_, painted) = fixture.frame(Vec::new());
    let texts = painted_texts(&painted);
    assert_eq!(
        count(&texts, "18 hidden lines"),
        0,
        "위로 20줄 펼침은 18줄 영역을 모두 엽니다"
    );
    assert_eq!(count(&texts, "line 30"), 2);
    assert_eq!(count(&texts, "15 hidden lines"), 2);

    let geometry = shown.modified.geometry.clone();
    let icon = pos2(
        (geometry.rect.left() + geometry.content_rect.left()) * HALF,
        bars[0].center().y,
    );
    fixture.press(icon);
    let (_, painted) = fixture.frame(Vec::new());
    let texts = painted_texts(&painted);
    assert!(
        !texts.iter().any(|text| text.ends_with("hidden lines")),
        "아이콘은 전부 펼칩니다"
    );
    assert_eq!(count(&texts, "line 05"), 2);
}
