use egui::{Color32, Context, FontId, RawInput, Rect, Ui, Vec2};
use taide_native_editor::documentation::{Block, Inline, RichDocument, Span};
use taide_native_ui::editor_markup::{Colors, Provider, show, show_preview};
use taide_native_ui::editor_surface::EditorAppearance;

const SCREEN: Vec2 = egui::vec2(400.0, 700.0);
const EDITOR_FONT_SIZE: f32 = 20.0;
const EDITOR_LINE_HEIGHT: f32 = 30.0;
const HEADING_SIZES: [f32; 3] = [24.0, 20.0, 18.0];
const PARAGRAPH_SIZE: f32 = 14.0;
const PARAGRAPH_LINE_HEIGHT: f32 = 22.75;
const QUOTE_BORDER_WIDTH: f32 = 2.0;
const CELL_BORDER_WIDTH: f32 = 1.0;
const TABLE_CELL_COUNT: usize = 2;

struct Preview;
impl Provider for Preview {
    fn open_link(&mut self, _: &Ui, _: &str) -> bool {
        false
    }
}

fn colors() -> Colors {
    Colors {
        background: Color32::BLACK,
        foreground: Color32::WHITE,
        border: Color32::GREEN,
        highlight: Color32::BLUE,
        link: Color32::BLUE,
        code_background: Color32::DARK_GRAY,
        shadow: Color32::BLACK,
    }
}

fn appearance() -> EditorAppearance {
    EditorAppearance {
        font: FontId::monospace(EDITOR_FONT_SIZE),
        line_height: EDITOR_LINE_HEIGHT,
        horizontal_padding: 0.0,
        background: Color32::BLACK,
        foreground: Color32::WHITE,
        muted: Color32::GRAY,
        selection: Color32::BLUE,
        cursor: Color32::WHITE,
        current_line: Color32::BLACK,
        line_numbers: false,
        indent: String::new(),
    }
}

fn text(value: &str) -> Vec<Inline> {
    vec![Inline::Text(Span {
        text: value.into(),
        ..Default::default()
    })]
}

fn table_edges(output: &egui::FullOutput) -> Vec<[egui::Pos2; 2]> {
    output
        .shapes
        .iter()
        .flat_map(|shape| match &shape.shape {
            egui::Shape::LineSegment { points, stroke }
                if stroke.width == CELL_BORDER_WIDTH && stroke.color == colors().border =>
            {
                vec![*points]
            }
            egui::Shape::Rect(rect)
                if rect.stroke.width == CELL_BORDER_WIDTH
                    && rect.stroke.color == colors().border =>
            {
                vec![
                    [rect.rect.left_top(), rect.rect.right_top()],
                    [rect.rect.left_bottom(), rect.rect.right_bottom()],
                    [rect.rect.left_top(), rect.rect.left_bottom()],
                    [rect.rect.right_top(), rect.rect.right_bottom()],
                ]
            }
            _ => Vec::new(),
        })
        .collect()
}

fn table_bounds(
    edges: &[[egui::Pos2; 2]],
    column_count: usize,
    row_count: usize,
) -> (Vec<f32>, Vec<f32>) {
    let mut columns = edges
        .iter()
        .filter_map(|points| (points[0].x == points[1].x).then_some(points[0].x))
        .collect::<Vec<_>>();
    let mut rows = edges
        .iter()
        .filter_map(|points| (points[0].y == points[1].y).then_some(points[0].y))
        .collect::<Vec<_>>();
    columns.sort_by(f32::total_cmp);
    columns.dedup();
    rows.sort_by(f32::total_cmp);
    rows.dedup();
    assert_eq!(
        columns.len(),
        column_count + 1,
        "columns={columns:?}; rows={rows:?}"
    );
    assert_eq!(
        rows.len(),
        row_count + 1,
        "columns={columns:?}; rows={rows:?}"
    );
    for column in columns.windows(2) {
        for row in rows.windows(2) {
            for points in [
                [egui::pos2(column[0], row[0]), egui::pos2(column[1], row[0])],
                [egui::pos2(column[0], row[1]), egui::pos2(column[1], row[1])],
                [egui::pos2(column[0], row[0]), egui::pos2(column[0], row[1])],
                [egui::pos2(column[1], row[0]), egui::pos2(column[1], row[1])],
            ] {
                assert_eq!(edges.iter().filter(|edge| **edge == points).count(), 1);
            }
        }
    }
    (columns, rows)
}

#[test]
fn 미리보기_제목과_본문은_편집기_글꼴_크기와_별개의_원본_크기를_사용한다() {
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![
            Block::Heading {
                level: 1,
                contents: text("heading-one"),
            },
            Block::Heading {
                level: 2,
                contents: text("heading-two"),
            },
            Block::Heading {
                level: 3,
                contents: text("heading-three"),
            },
            Block::Paragraph(text("paragraph")),
        ],
    };
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| {
            show_preview(ui, &document, &appearance(), colors(), &mut Preview);
        },
    );
    output.textures_delta.clear();
    for (label, size) in ["heading-one", "heading-two", "heading-three"]
        .into_iter()
        .zip(HEADING_SIZES)
    {
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == label && text.galley.job.sections.iter().all(|section| section.format.font_id.size == size))));
    }
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "paragraph" && text.galley.job.sections.iter().all(|section| section.format.font_id.size == PARAGRAPH_SIZE && section.format.line_height == Some(PARAGRAPH_LINE_HEIGHT)))));
}

#[test]
fn 인용과_표는_원본_테두리를_사용하고_긴_코드는_줄바꿈하지_않고_가로로_스크롤한다() {
    const CODE_REPEAT: usize = 100;
    const SCROLL_DELTA: f32 = 120.0;
    const FRAME_SECONDS: f64 = 1.0 / 60.0;
    let context = Context::default();
    let code = "long_code_".repeat(CODE_REPEAT);
    let document = RichDocument {
        blocks: vec![
            Block::Quote(vec![Block::Paragraph(text("quote"))]),
            Block::Table {
                columns: vec![],
                header: vec![text("header")],
                rows: vec![vec![text("value")]],
            },
            Block::Code {
                language: String::new(),
                text: code.clone(),
            },
        ],
    };
    let paint = |events| {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(context.cumulative_frame_nr() as f64 * FRAME_SECONDS),
                events,
                ..Default::default()
            },
            |ui| {
                show_preview(ui, &document, &appearance(), colors(), &mut Preview);
            },
        );
        output.textures_delta.clear();
        output
    };
    let _ = paint(Vec::new());
    let output = paint(Vec::new());
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::LineSegment { points, stroke } if points[0].x == points[1].x && stroke.width == QUOTE_BORDER_WIDTH)));
    assert!(table_edges(&output).len() >= TABLE_CELL_COUNT);
    let (before, point) = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == code => {
                assert_eq!(text.galley.rows.len(), 1);
                assert!(text.galley.size().x > shape.clip_rect.width());
                Some((
                    text.pos.x,
                    egui::pos2(
                        shape.clip_rect.center().x,
                        text.pos.y + text.galley.size().y / 2.0,
                    ),
                ))
            }
            _ => None,
        })
        .unwrap();
    let _ = paint(vec![
        egui::Event::PointerMoved(point),
        egui::Event::MouseWheel {
            phase: egui::TouchPhase::Move,
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(-SCROLL_DELTA, 0.0),
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    let output = paint(Vec::new());
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == code && text.pos.x < before)));
}

#[test]
fn 인라인_코드_프레임은_원본_둥근_배경과_링크_클릭을_보존한다() {
    const CODE_RADIUS: u8 = 4;
    const TARGET: &str = "https://example.com/code";
    #[derive(Default)]
    struct Links {
        targets: Vec<String>,
    }
    impl Provider for Links {
        fn open_link(&mut self, _: &Ui, target: &str) -> bool {
            self.targets.push(target.into());
            true
        }
    }
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Paragraph(vec![Inline::Text(Span {
            text: "linked-code".into(),
            style: taide_native_editor::documentation::Style {
                code: true,
                ..Default::default()
            },
            link: Some(taide_native_editor::documentation::Link {
                target: TARGET.into(),
                title: String::new(),
            }),
        })])],
    };
    let mut links = Links::default();
    let mut paint = |events| {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| {
                show_preview(ui, &document, &appearance(), colors(), &mut links);
            },
        );
        output.textures_delta.clear();
        output
    };
    let output = paint(Vec::new());
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == colors().code_background && rect.corner_radius.nw == CODE_RADIUS)));
    let position = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "linked-code" => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap();
    for pressed in [true, false] {
        let _ = paint(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
    }
    assert_eq!(links.targets, [TARGET]);
}

#[test]
fn 표의_서로_다른_본문_폭과_높이에서도_모든_셀_경계는_같은_열과_행에_맞는다() {
    const WARMUP_FRAMES: usize = 1;
    const COLUMN_COUNT: usize = 2;
    const ROW_COUNT: usize = 3;
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![text("short"), text("header-two")],
            rows: vec![
                vec![
                    text("longer content in column one"),
                    vec![
                        Inline::Text(Span {
                            text: "first".into(),
                            ..Default::default()
                        }),
                        Inline::Break,
                        Inline::Text(Span {
                            text: "second".into(),
                            ..Default::default()
                        }),
                    ],
                ],
                vec![text("one"), text("last")],
            ],
        }],
    };
    let mut edges = Vec::new();
    for _ in 0..WARMUP_FRAMES {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                show_preview(ui, &document, &appearance(), colors(), &mut Preview);
            },
        );
        output.textures_delta.clear();
        edges = table_edges(&output);
    }
    table_bounds(&edges, COLUMN_COUNT, ROW_COUNT);
}

#[test]
fn 표의_내용과_viewport_변경은_첫_프레임부터_본문과_경계를_일치시킨다() {
    const NARROW_WIDTH: f32 = 240.0;
    const WIDE_WIDTH: f32 = 560.0;
    const TOLERANCE: f32 = 1.0;
    let context = Context::default();
    for (width, labels) in [
        (
            SCREEN.x,
            vec![
                vec!["first header is wider", "right header"],
                vec!["left body", "right body"],
            ],
        ),
        (
            SCREEN.x,
            vec![
                vec!["left", "right"],
                vec!["a", "b"],
                vec!["last left", "last right"],
            ],
        ),
        (
            NARROW_WIDTH,
            vec![
                vec!["narrow header", "other header"],
                vec![
                    "many words that wrap in a narrow viewport and remain readable",
                    "a shorter paragraph",
                ],
            ],
        ),
        (
            WIDE_WIDTH,
            vec![
                vec!["wide header", "final header"],
                vec!["changed body becomes wider again", "final paragraph"],
            ],
        ),
    ] {
        let document = RichDocument {
            blocks: vec![Block::Table {
                columns: vec![],
                header: labels[0].iter().map(|label| text(label)).collect(),
                rows: labels
                    .iter()
                    .skip(1)
                    .map(|row| row.iter().map(|label| text(label)).collect())
                    .collect(),
            }],
        };
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, SCREEN.y),
                )),
                ..Default::default()
            },
            |ui| show_preview(ui, &document, &appearance(), colors(), &mut Preview),
        );
        output.textures_delta.clear();
        let (columns, rows) = table_bounds(&table_edges(&output), labels[0].len(), labels.len());
        for (row, labels) in labels.iter().enumerate() {
            for (column, label) in labels.iter().enumerate() {
                let text = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.job.text == *label => Some(text),
                        _ => None,
                    })
                    .unwrap();
                let rect = Rect::from_min_size(text.pos, text.galley.size());
                assert!(
                    rect.left() + TOLERANCE >= columns[column],
                    "left boundary: {label}"
                );
                assert!(
                    rect.right() <= columns[column + 1] + TOLERANCE,
                    "right boundary: {label}"
                );
                assert!(rect.top() + TOLERANCE >= rows[row], "top boundary: {label}");
                assert!(
                    rect.bottom() <= rows[row + 1] + TOLERANCE,
                    "bottom boundary: {label}"
                );
            }
        }
    }
}

#[test]
fn 표의_이미지_준비와_높이_변경은_경계와_링크_id_캐시_크기를_보존한다() {
    const INITIAL_SIZE: Vec2 = egui::vec2(40.0, 20.0);
    const TALL_SIZE: Vec2 = egui::vec2(40.0, 100.0);
    const TALL_INDEX: usize = 2;
    const TARGET: &str = "https://example.com/table-image";
    const TOLERANCE: f32 = 1.0;
    struct Images {
        size: Option<Vec2>,
        rects: Vec<Rect>,
        opened: Vec<String>,
    }
    impl Provider for Images {
        fn open_link(&mut self, _: &Ui, target: &str) -> bool {
            self.opened.push(target.into());
            true
        }
        fn image_size(
            &self,
            _: &str,
            _: taide_native_editor::documentation::ImageDimensions,
        ) -> Option<Vec2> {
            self.size
        }
        fn image(
            &mut self,
            ui: &mut Ui,
            _: &str,
            _: &str,
            _: taide_native_editor::documentation::ImageDimensions,
        ) -> Option<egui::Response> {
            let size = self.size?;
            let size = size * (ui.available_width().max(0.0) / size.x).min(1.0);
            let response = ui.allocate_exact_size(size, egui::Sense::click()).1;
            if ui.is_visible() {
                self.rects.push(response.rect)
            }
            Some(response)
        }
    }
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![text("label"), text("picture")],
            rows: vec![vec![
                text("content"),
                vec![Inline::Image {
                    source: "synthetic-image".into(),
                    title: String::new(),
                    alt: "waiting image".into(),
                    link: Some(taide_native_editor::documentation::Link {
                        target: TARGET.into(),
                        title: String::new(),
                    }),
                    dimensions: Default::default(),
                }],
            ]],
        }],
    };
    let mut provider = Images {
        size: None,
        rects: Vec::new(),
        opened: Vec::new(),
    };
    let paint = |provider: &mut Images, events| {
        provider.rects.clear();
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| show_preview(ui, &document, &appearance(), colors(), provider),
        );
        output.textures_delta.clear();
        output
    };
    let mut heights = Vec::new();
    let mut memory = None;
    for (index, size) in [
        None,
        Some(INITIAL_SIZE),
        Some(TALL_SIZE),
        Some(INITIAL_SIZE),
    ]
    .into_iter()
    .enumerate()
    {
        provider.size = size;
        let output = paint(&mut provider, Vec::new());
        let (columns, rows) = table_bounds(&table_edges(&output), 2, 2);
        heights.push(rows.last().unwrap() - rows[1]);
        if let Some(size) = size {
            assert_eq!(provider.rects.len(), 1);
            let rect = provider.rects[0];
            assert!(
                (rect.width() - rect.height() * size.x / size.y).abs() <= TOLERANCE,
                "image rect={rect:?}; requested={size:?}"
            );
            assert!(rect.left() + TOLERANCE >= columns[1]);
            assert!(rect.right() <= columns.last().unwrap() + TOLERANCE);
            assert!(rect.top() + TOLERANCE >= rows[1]);
            assert!(rect.bottom() <= rows.last().unwrap() + TOLERANCE);
        }
        if index == 1 {
            memory = Some(context.data(|data| data.len()))
        }
    }
    assert!(heights[TALL_INDEX] > heights[1]);
    assert_eq!(heights.last().unwrap(), &heights[1]);
    assert_eq!(memory.unwrap(), context.data(|data| data.len()));
    let position = provider.rects[0].center();
    for pressed in [true, false] {
        paint(
            &mut provider,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(provider.opened, [TARGET]);
}

#[test]
fn 표는_내용의_자연_너비를_쓰고_공간이_부족할_때만_단어를_줄바꿈한다() {
    const NARROW_WIDTH: f32 = 240.0;
    const WIDE_WIDTH: f32 = 560.0;
    const BODY: &str = "many words remain on one line in a wide viewport";
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![text("body"), text("state")],
            rows: vec![vec![text(BODY), text("ready")]],
        }],
    };
    for width in [WIDE_WIDTH, NARROW_WIDTH, WIDE_WIDTH] {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, SCREEN.y),
                )),
                ..Default::default()
            },
            |ui| show_preview(ui, &document, &appearance(), colors(), &mut Preview),
        );
        output.textures_delta.clear();
        let galley = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == BODY => Some(&text.galley),
                _ => None,
            })
            .unwrap();
        if width == WIDE_WIDTH {
            assert_eq!(galley.rows.len(), 1);
        } else {
            assert!(galley.rows.len() > 1);
        }
    }
}

#[test]
fn 표의_최소_너비는_공백없는_단어를_보존하고_한중일_문장을_줄바꿈한다() {
    const NARROW_WIDTH: f32 = 240.0;
    const WORD_REPEAT: usize = 40;
    const CJK_REPEAT: usize = 8;
    let long_word = "unbroken".repeat(WORD_REPEAT);
    for (body, should_wrap) in [
        (long_word, false),
        ("한글문장".repeat(CJK_REPEAT), true),
        ("日本語文".repeat(CJK_REPEAT), true),
        ("汉字句子".repeat(CJK_REPEAT), true),
    ] {
        let context = Context::default();
        let document = RichDocument {
            blocks: vec![Block::Table {
                columns: vec![],
                header: vec![text("body"), text("state")],
                rows: vec![vec![text(&body), text("ready")]],
            }],
        };
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(NARROW_WIDTH, SCREEN.y),
                )),
                ..Default::default()
            },
            |ui| show_preview(ui, &document, &appearance(), colors(), &mut Preview),
        );
        output.textures_delta.clear();
        let galley = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == body => Some(&text.galley),
                _ => None,
            })
            .unwrap();
        assert_eq!(galley.rows.len() > 1, should_wrap, "body={body}");
        if !should_wrap {
            let edges = table_edges(&output);
            assert!(edges.iter().any(|points| points[0].x > NARROW_WIDTH));
        }
    }
}

#[test]
fn 순서_목록은_음수에서_0을_거쳐_증가하고_큰_번호에서도_overflow하지_않는다() {
    const NEGATIVE_START: i64 = -1;
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::List {
            start: Some(NEGATIVE_START),
            items: vec![
                taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: None,
                    blocks: vec![Block::Paragraph(text("first"))],
                },
                taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: None,
                    blocks: vec![Block::Paragraph(text("second"))],
                },
                taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: None,
                    blocks: vec![Block::Paragraph(text("third"))],
                },
            ],
        }],
    };
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| {
            show_preview(ui, &document, &appearance(), colors(), &mut Preview);
        },
    );
    output.textures_delta.clear();
    for number in ["-1.", "0.", "1."] {
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == number)));
    }
    assert_eq!(
        taide_native_editor::documentation::list_ordinal(i64::MAX, 1),
        i64::MAX
    );
}

#[test]
fn 항목별_ordinal은_미리보기와_일반_표시에서_역순과_번호_재지정을_보존한다() {
    const ORDINALS: [i64; 3] = [3, -4, -5];
    let document = RichDocument {
        blocks: vec![Block::List {
            start: Some(ORDINALS[0]),
            items: ORDINALS
                .into_iter()
                .map(|ordinal| taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: Some(ordinal),
                    blocks: vec![Block::Paragraph(text("item"))],
                })
                .collect(),
        }],
    };
    for is_preview in [false, true] {
        let context = Context::default();
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                if is_preview {
                    show_preview(ui, &document, &appearance(), colors(), &mut Preview);
                    return;
                }
                show(ui, &document, &appearance(), colors(), &mut Preview);
            },
        );
        output.textures_delta.clear();
        for ordinal in ORDINALS {
            let number = format!("{ordinal}.");
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == number)));
        }
    }
}
