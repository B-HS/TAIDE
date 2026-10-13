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
fn 미리보기는_암시_줄과_컨테이너_여백을_원본처럼_배치한다() {
    use taide_native_editor::documentation::ListItem;
    const EXPECTED_LINE_HEIGHT: f32 = 20.0;
    const EXPECTED_MARGIN: f32 = 8.0;
    let items = ["plain first", "plain second"]
        .into_iter()
        .map(|label| ListItem {
            checked: None,
            ordinal: None,
            marker_style: None,
            blocks: vec![Block::Inline(text(label))],
        })
        .collect();
    let document = RichDocument {
        blocks: vec![
            Block::List {
                start: None,
                items,
                style: Default::default(),
            },
            Block::Quote(vec![
                Block::Paragraph(text("quote first")),
                Block::Paragraph(text("quote second")),
            ]),
        ],
    };
    let context = Context::default();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| show_preview(ui, &document, &appearance(), colors(), &mut Preview),
    );
    output.textures_delta.clear();
    let rect = |label| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(Rect::from_min_size(text.pos, text.galley.size()))
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(rect("plain first").height(), EXPECTED_LINE_HEIGHT);
    assert_eq!(
        rect("plain second").top() - rect("plain first").bottom(),
        0.0
    );
    assert_eq!(
        rect("quote second").top() - rect("quote first").bottom(),
        EXPECTED_MARGIN
    );
    let border = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::LineSegment { points, stroke } if stroke.width == QUOTE_BORDER_WIDTH => {
                Some(Rect::from_two_pos(points[0], points[1]))
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(border.top(), rect("quote first").top());
    assert_eq!(border.bottom(), rect("quote second").bottom());
}

#[test]
fn 미리보기_코드와_상속_굵기는_편집기_글꼴과_별개로_선택된다() {
    use taide_native_editor::documentation::{ListItem, Style};
    use taide_native_ui::font_families::{
        BOLD_FAMILY, CODE_BOLD_FAMILY, CODE_SEMIBOLD_FAMILY, SEMIBOLD_FAMILY,
    };
    let context = Context::default();
    let mut definitions = egui::FontDefinitions::default();
    for name in [BOLD_FAMILY, SEMIBOLD_FAMILY] {
        definitions.families.insert(
            egui::FontFamily::Name(name.into()),
            definitions.families[&egui::FontFamily::Proportional].clone(),
        );
    }
    for name in [CODE_BOLD_FAMILY, CODE_SEMIBOLD_FAMILY] {
        definitions.families.insert(
            egui::FontFamily::Name(name.into()),
            definitions.families[&egui::FontFamily::Monospace].clone(),
        );
    }
    context.set_fonts(definitions);
    let code = |label: &str, is_bold| {
        vec![Inline::Text(Span {
            text: label.into(),
            style: Style {
                code: true,
                bold: is_bold,
                ..Default::default()
            },
            ..Default::default()
        })]
    };
    let document = RichDocument {
        blocks: vec![
            Block::Paragraph(code("inline code", false)),
            Block::Paragraph(code("strong code", true)),
            Block::Heading {
                level: 1,
                contents: code("heading code", false),
            },
            Block::Heading {
                level: 1,
                contents: text("heading text"),
            },
            Block::Code {
                language: String::new(),
                text: "header code".into(),
                style: Style {
                    bold: true,
                    ..Default::default()
                },
            },
            Block::List {
                start: Some(1),
                items: vec![ListItem {
                    checked: None,
                    ordinal: None,
                    marker_style: None,
                    blocks: vec![Block::Inline(text("list text"))],
                }],
                style: Style {
                    bold: true,
                    ..Default::default()
                },
            },
        ],
    };
    let mut appearance = appearance();
    appearance.font = FontId::new(
        EDITOR_FONT_SIZE,
        egui::FontFamily::Name("unregistered-editor-setting".into()),
    );
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| show_preview(ui, &document, &appearance, colors(), &mut Preview),
    );
    output.textures_delta.clear();
    for (label, family) in [
        ("inline code", egui::FontFamily::Monospace),
        (
            "strong code",
            egui::FontFamily::Name(CODE_BOLD_FAMILY.into()),
        ),
        (
            "heading code",
            egui::FontFamily::Name(CODE_SEMIBOLD_FAMILY.into()),
        ),
        (
            "heading text",
            egui::FontFamily::Name(SEMIBOLD_FAMILY.into()),
        ),
        (
            "header code",
            egui::FontFamily::Name(CODE_BOLD_FAMILY.into()),
        ),
        ("1.", egui::FontFamily::Name(BOLD_FAMILY.into())),
    ] {
        let text = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
                _ => None,
            })
            .unwrap();
        assert!(
            text.galley
                .job
                .sections
                .iter()
                .all(|section| section.format.font_id.family == family),
            "wrong family {label}"
        );
    }
}

#[test]
fn 인접_문단의_세로_여백은_원본처럼_병합된다() {
    const EXPECTED_GAP: f32 = 8.0;
    let document = RichDocument {
        blocks: vec![
            Block::Paragraph(text("first flow paragraph")),
            Block::Paragraph(text("second flow paragraph")),
        ],
    };
    let context = Context::default();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| show_preview(ui, &document, &appearance(), colors(), &mut Preview),
    );
    output.textures_delta.clear();
    let first = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "first flow paragraph" => {
                Some(Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap();
    let second = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "second flow paragraph" => {
                Some(Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(second.top() - first.bottom(), EXPECTED_GAP);
}

#[test]
fn 복합_셀의_블록과_중첩_표는_병합_영역과_현재_너비에_맞게_배치된다() {
    use taide_native_editor::documentation::{Alignment, ListItem, TableCell, TableLayout};
    const COLUMN_COUNT: usize = 2;
    const ROW_COUNT: usize = 2;
    const TOLERANCE: f32 = 1.0;
    const WIDTHS: [f32; 4] = [240.0, 520.0, 240.0, 520.0];
    const FIRST: &str = "first paragraph with enough words to wrap within the cell";
    let nested = Block::Table {
        columns: vec![],
        header: vec![],
        rows: vec![vec![text("nested table")]],
        layout: Some(TableLayout {
            caption_blocks: None,
            caption: vec![],
            column_count: 1,
            row_count: 1,
            cells: vec![vec![TableCell {
                column: 0,
                column_span: 1,
                row_span: 1,
                alignment: Alignment::Default,
                blocks: None,
            }]],
        }),
    };
    let body = vec![
        Block::Paragraph(text(FIRST)),
        Block::Paragraph(text("second paragraph")),
        Block::Heading {
            level: 1,
            contents: text("cell heading"),
        },
        Block::List {
            style: Default::default(),
            start: Some(-1),
            items: vec![ListItem {
                checked: None,
                ordinal: None,
                marker_style: None,
                blocks: vec![Block::Paragraph(text("list item"))],
            }],
        },
        Block::Code {
            style: Default::default(),
            language: String::new(),
            text: "let cell = 1;\nnext line".into(),
        },
        nested,
    ];
    let mut document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![],
            rows: vec![
                vec![vec![], text("neighbor")],
                vec![text("bottom neighbor")],
            ],
            layout: Some(TableLayout {
                caption_blocks: None,
                caption: vec![],
                column_count: COLUMN_COUNT,
                row_count: ROW_COUNT,
                cells: vec![
                    vec![
                        TableCell {
                            column: 0,
                            column_span: 1,
                            row_span: ROW_COUNT,
                            alignment: Alignment::Default,
                            blocks: Some(body),
                        },
                        TableCell {
                            column: 1,
                            column_span: 1,
                            row_span: 1,
                            alignment: Alignment::Default,
                            blocks: None,
                        },
                    ],
                    vec![TableCell {
                        column: 1,
                        column_span: 1,
                        row_span: 1,
                        alignment: Alignment::Default,
                        blocks: None,
                    }],
                ],
            }),
        }],
    };
    let context = Context::default();
    let mut state_count = None;
    let mut paragraph_rows = Vec::new();
    let mut previous_height = 0.0;
    for width in WIDTHS {
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
        let edges = table_edges(&output);
        let bounds = edges
            .iter()
            .map(|points| Rect::from_two_pos(points[0], points[1]))
            .reduce(|left, right| left.union(right))
            .unwrap()
            .expand(TOLERANCE);
        previous_height = bounds.height();
        let mut previous_bottom = bounds.top();
        for label in [
            FIRST,
            "second paragraph",
            "cell heading",
            "list item",
            "let cell = 1;\nnext line",
            "nested table",
        ] {
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
                    _ => None,
                })
                .unwrap();
            let rect = Rect::from_min_size(text.pos, text.galley.size());
            assert!(bounds.contains_rect(rect), "outside table {label}");
            assert!(
                rect.top() + TOLERANCE >= previous_bottom,
                "overlapping block {label}"
            );
            previous_bottom = rect.bottom();
            if label == FIRST {
                paragraph_rows.push(text.galley.rows.len())
            }
            if label == "cell heading" {
                assert!(
                    text.galley
                        .job
                        .sections
                        .iter()
                        .all(|section| section.format.font_id.size == HEADING_SIZES[0])
                )
            }
        }
        for label in ["-1.", "neighbor", "bottom neighbor"] {
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == label)));
        }
        let count = context.data(|data| data.len());
        if let Some(previous) = state_count {
            assert_eq!(count, previous)
        }
        state_count = Some(count);
    }
    assert!(paragraph_rows[0] > paragraph_rows[1]);
    assert_eq!(paragraph_rows[0], paragraph_rows[2]);
    assert_eq!(paragraph_rows[1], paragraph_rows[3]);
    for is_added in [true, false] {
        let Block::Table {
            layout: Some(layout),
            ..
        } = &mut document.blocks[0]
        else {
            panic!("expected table")
        };
        let blocks = layout.cells[0][0].blocks.as_mut().unwrap();
        if is_added {
            blocks.push(Block::Paragraph(text("inserted block")))
        } else {
            blocks.pop();
        }
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(WIDTHS[1], SCREEN.y),
                )),
                ..Default::default()
            },
            |ui| show_preview(ui, &document, &appearance(), colors(), &mut Preview),
        );
        output.textures_delta.clear();
        let edges = table_edges(&output);
        let bounds = edges
            .iter()
            .map(|points| Rect::from_two_pos(points[0], points[1]))
            .reduce(|left, right| left.union(right))
            .unwrap()
            .expand(TOLERANCE);
        assert_eq!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "inserted block")), is_added);
        if is_added {
            assert!(bounds.height() > previous_height)
        } else {
            assert!((bounds.height() - previous_height).abs() <= TOLERANCE)
        }
    }
    const CHECKBOX_INTERACT_SIZE: f32 = 60.0;
    let Block::Table {
        layout: Some(layout),
        ..
    } = &mut document.blocks[0]
    else {
        panic!("expected table")
    };
    layout.cells[0][0].blocks = Some(vec![Block::List {
        style: Default::default(),
        start: None,
        items: vec![ListItem {
            checked: Some(true),
            ordinal: None,
            marker_style: None,
            blocks: vec![Block::Paragraph(text("task item"))],
        }],
    }]);
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(WIDTHS[1], SCREEN.y),
            )),
            ..Default::default()
        },
        |ui| {
            ui.spacing_mut().interact_size.y = CHECKBOX_INTERACT_SIZE;
            show_preview(ui, &document, &appearance(), colors(), &mut Preview);
        },
    );
    output.textures_delta.clear();
    let mut columns = table_edges(&output)
        .iter()
        .filter_map(|points| (points[0].x == points[1].x).then_some(points[0].x))
        .collect::<Vec<_>>();
    columns.sort_by(f32::total_cmp);
    columns.dedup();
    assert_eq!(columns.len(), COLUMN_COUNT + 1);
    let text = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "task item" => Some(text),
            _ => None,
        })
        .unwrap();
    assert!(
        text.pos.x + text.galley.size().x <= columns[1] + TOLERANCE,
        "checkbox item exceeds cell"
    );
    assert_eq!(
        text.galley.rows.len(),
        1,
        "checkbox squeezes a naturally fitting item"
    );
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
                layout: None,
                columns: vec![],
                header: vec![text("header")],
                rows: vec![vec![text("value")]],
            },
            Block::Code {
                style: Default::default(),
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
            layout: None,
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
                layout: None,
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
    const CENTER_FRACTION: f32 = 0.5;
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
    for (is_html, has_blocks, has_caption) in [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (true, false, true),
    ] {
        let context = Context::default();
        let mut document = RichDocument {
            blocks: vec![Block::Table {
                layout: None,
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
        if is_html {
            let Block::Table {
                header,
                rows,
                layout,
                ..
            } = &mut document.blocks[0]
            else {
                panic!("expected table")
            };
            rows.insert(0, std::mem::take(header));
            *layout = Some(taide_native_editor::documentation::TableLayout {
                caption_blocks: None,
                caption: Vec::new(),
                column_count: rows[0].len(),
                row_count: rows.len(),
                cells: rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .map(
                                |(column, _)| taide_native_editor::documentation::TableCell {
                                    column,
                                    column_span: 1,
                                    row_span: 1,
                                    alignment:
                                        taide_native_editor::documentation::Alignment::Default,
                                    blocks: None,
                                },
                            )
                            .collect()
                    })
                    .collect(),
            });
            if has_blocks {
                layout.as_mut().unwrap().cells[1][1].blocks =
                    Some(vec![Block::Quote(vec![Block::Paragraph(std::mem::take(
                        &mut rows[1][1],
                    ))])]);
            }
            if has_caption {
                layout.as_mut().unwrap().caption_blocks =
                    Some(vec![Block::Paragraph(std::mem::take(&mut rows[1][1]))]);
            }
        }
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
            let edges = table_edges(&output);
            let (columns, rows) = if is_html {
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
                (columns, rows)
            } else {
                table_bounds(&edges, 2, 2)
            };
            heights.push(if has_caption {
                rows[0]
            } else {
                rows.last().unwrap() - rows[1]
            });
            if let Some(size) = size {
                assert_eq!(provider.rects.len(), 1);
                let rect = provider.rects[0];
                assert!(
                    (rect.width() - rect.height() * size.x / size.y).abs() <= TOLERANCE,
                    "image rect={rect:?}; requested={size:?}"
                );
                assert!(rect.left() + TOLERANCE >= columns[usize::from(!has_caption)]);
                assert!(rect.right() <= columns.last().unwrap() + TOLERANCE);
                if has_caption {
                    assert!(rect.bottom() <= rows[0] + TOLERANCE);
                    assert!(
                        (rect.center().x
                            - (columns[0] + columns.last().unwrap()) * CENTER_FRACTION)
                            .abs()
                            <= TOLERANCE
                    );
                } else {
                    assert!(rect.top() + TOLERANCE >= rows[1]);
                    assert!(rect.bottom() <= rows.last().unwrap() + TOLERANCE);
                }
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
}

#[test]
fn 표는_내용의_자연_너비를_쓰고_공간이_부족할_때만_단어를_줄바꿈한다() {
    const NARROW_WIDTH: f32 = 240.0;
    const WIDE_WIDTH: f32 = 560.0;
    const BODY: &str = "many words remain on one line in a wide viewport";
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            layout: None,
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
                layout: None,
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
fn 병합_셀의_내부_경계는_없고_제목과_본문은_병합_영역에_배치된다() {
    use taide_native_editor::documentation::{Alignment, TableCell, TableLayout};
    const COLUMN_COUNT: usize = 3;
    const ROW_COUNT: usize = 2;
    const MERGED_SPAN: usize = 2;
    const EDGE_COUNT: usize = 7;
    const TOLERANCE: f32 = 1.0;
    const NARROW_WIDTH: f32 = 180.0;
    const WIDE_WIDTH: f32 = 600.0;
    const CENTER_FRACTION: f32 = 0.5;
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![],
            rows: vec![
                vec![text("wide"), text("tall")],
                vec![text("first"), text("second")],
            ],
            layout: Some(TableLayout {
                caption_blocks: None,
                caption: text("table caption"),
                column_count: COLUMN_COUNT,
                row_count: ROW_COUNT,
                cells: vec![
                    vec![
                        TableCell {
                            column: 0,
                            column_span: MERGED_SPAN,
                            row_span: 1,
                            alignment: Alignment::Center,
                            blocks: None,
                        },
                        TableCell {
                            column: MERGED_SPAN,
                            column_span: 1,
                            row_span: MERGED_SPAN,
                            alignment: Alignment::Default,
                            blocks: None,
                        },
                    ],
                    vec![
                        TableCell {
                            column: 0,
                            column_span: 1,
                            row_span: 1,
                            alignment: Alignment::Default,
                            blocks: None,
                        },
                        TableCell {
                            column: 1,
                            column_span: 1,
                            row_span: 1,
                            alignment: Alignment::Default,
                            blocks: None,
                        },
                    ],
                ],
            }),
        }],
    };
    let mut memory = None;
    for width in [SCREEN.x, NARROW_WIDTH, WIDE_WIDTH, SCREEN.x] {
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
        let edges = table_edges(&output);
        assert_eq!(edges.len(), EDGE_COUNT, "edges={edges:?}");
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
        assert_eq!(columns.len(), COLUMN_COUNT + 1);
        assert_eq!(rows.len(), ROW_COUNT + 1);
        for points in [
            [
                egui::pos2(columns[0], rows[0]),
                egui::pos2(columns[COLUMN_COUNT], rows[0]),
            ],
            [
                egui::pos2(columns[0], rows[1]),
                egui::pos2(columns[MERGED_SPAN], rows[1]),
            ],
            [
                egui::pos2(columns[0], rows[ROW_COUNT]),
                egui::pos2(columns[COLUMN_COUNT], rows[ROW_COUNT]),
            ],
            [
                egui::pos2(columns[1], rows[1]),
                egui::pos2(columns[1], rows[ROW_COUNT]),
            ],
        ] {
            assert_eq!(edges.iter().filter(|edge| **edge == points).count(), 1)
        }
        let bounds = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(Rect::from_min_size(text.pos, text.galley.size()))
                    }
                    _ => None,
                })
                .unwrap()
        };
        let caption = bounds("table caption");
        assert!(caption.bottom() <= rows[0] + TOLERANCE);
        assert!(
            (caption.center().x - (columns[0] + columns[COLUMN_COUNT]) * CENTER_FRACTION).abs()
                <= TOLERANCE
        );
        let wide = bounds("wide");
        assert!(
            (wide.center().x - (columns[0] + columns[MERGED_SPAN]) * CENTER_FRACTION).abs()
                <= TOLERANCE
        );
        let tall = bounds("tall");
        assert!(
            (tall.center().y - (rows[0] + rows[ROW_COUNT]) * CENTER_FRACTION).abs() <= TOLERANCE
        );
        for (label, column) in [("first", 0), ("second", 1)] {
            let rect = bounds(label);
            assert!(
                rect.left() + TOLERANCE >= columns[column]
                    && rect.right() <= columns[column + 1] + TOLERANCE
            );
            assert!(
                rect.top() + TOLERANCE >= rows[1] && rect.bottom() <= rows[ROW_COUNT] + TOLERANCE
            );
        }
        let count = context.data(|data| data.len());
        if let Some(memory) = memory {
            assert_eq!(count, memory)
        }
        memory = Some(count);
    }
}

#[test]
fn 미리보기_문단의_이미지는_앞뒤_텍스트와_독립된_블록으로_표시된다() {
    use taide_native_editor::documentation::ImageDimensions;
    const IMAGE: Vec2 = egui::vec2(40.0, 20.0);
    const EXPECTED_FLOW_HEIGHT: f32 = 65.5;
    const TOLERANCE: f32 = 1.0;
    struct Images(Option<Rect>);
    impl Provider for Images {
        fn open_link(&mut self, _: &Ui, _: &str) -> bool {
            false
        }
        fn image_size(&self, _: &str, _: ImageDimensions) -> Option<Vec2> {
            Some(IMAGE)
        }
        fn image(
            &mut self,
            ui: &mut Ui,
            _: &str,
            _: &str,
            _: ImageDimensions,
        ) -> Option<egui::Response> {
            let response = ui.allocate_exact_size(IMAGE, egui::Sense::hover()).1;
            self.0 = Some(response.rect);
            Some(response)
        }
    }
    let document = RichDocument {
        blocks: vec![Block::Paragraph(vec![
            Inline::Text(Span {
                text: "before".into(),
                ..Default::default()
            }),
            Inline::Image {
                source: "synthetic".into(),
                title: String::new(),
                alt: "image".into(),
                link: None,
                dimensions: Default::default(),
            },
            Inline::Text(Span {
                text: "after".into(),
                ..Default::default()
            }),
        ])],
    };
    let context = Context::default();
    let mut images = Images(None);
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| show_preview(ui, &document, &appearance(), colors(), &mut images),
    );
    output.textures_delta.clear();
    let bounds = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .unwrap()
    };
    let image = images.0.unwrap();
    assert!(bounds("before").bottom() <= image.top());
    assert!(bounds("after").top() >= image.bottom());
    assert!(
        (bounds("after").bottom() - bounds("before").top() - EXPECTED_FLOW_HEIGHT).abs()
            <= TOLERANCE
    );
}

#[test]
fn 목록_본문은_마커_너비와_독립된_들여쓰기와_제목_정렬을_사용한다() {
    use taide_native_editor::documentation::{Alignment, ListItem, TableCell, TableLayout};
    const START: i64 = 9883;
    const INDENT: f32 = 20.0;
    const TASK_ICON_SIZE: f32 = 13.0;
    const CENTER_FRACTION: f32 = 0.5;
    const TOLERANCE: f32 = 1.0;
    let list = |label: &str, start| Block::List {
        start,
        items: vec![ListItem {
            checked: None,
            ordinal: None,
            marker_style: None,
            blocks: vec![Block::Inline(text(label))],
        }],
        style: Default::default(),
    };
    let document = RichDocument {
        blocks: vec![
            Block::Table {
                columns: vec![],
                header: vec![],
                rows: vec![vec![text(
                    "wide body forces the caption to use a common column width",
                )]],
                layout: Some(TableLayout {
                    caption: vec![],
                    caption_blocks: Some(vec![
                        list("centered item", Some(START)),
                        Block::List {
                            start: None,
                            items: vec![ListItem {
                                checked: Some(false),
                                ordinal: None,
                                marker_style: None,
                                blocks: vec![Block::Inline(text("task item"))],
                            }],
                            style: Default::default(),
                        },
                        Block::Code {
                            language: String::new(),
                            text: "short\nlonger code line".into(),
                            style: Default::default(),
                        },
                    ]),
                    cells: vec![vec![TableCell {
                        column: 0,
                        column_span: 1,
                        row_span: 1,
                        alignment: Alignment::Default,
                        blocks: None,
                    }]],
                    column_count: 1,
                    row_count: 1,
                }),
            },
            Block::Inline(text("origin")),
            list("short marker", Some(1)),
            list("wide marker", Some(START)),
        ],
    };
    let extra = [
        (
            3,
            "paragraph baseline",
            Block::Paragraph(text("paragraph baseline")),
        ),
        (
            4,
            "heading baseline",
            Block::Heading {
                level: 1,
                contents: text("heading baseline"),
            },
        ),
        (
            5,
            "code baseline",
            Block::Code {
                language: String::new(),
                text: "code baseline".into(),
                style: Default::default(),
            },
        ),
    ];
    let baseline_labels = extra
        .iter()
        .map(|(number, label, _)| (format!("{number}."), *label))
        .collect::<Vec<_>>();
    let document = RichDocument {
        blocks: document
            .blocks
            .into_iter()
            .chain(extra.into_iter().map(|(start, _, block)| Block::List {
                start: Some(start),
                items: vec![ListItem {
                    checked: None,
                    ordinal: None,
                    marker_style: None,
                    blocks: vec![block],
                }],
                style: Default::default(),
            }))
            .collect(),
    };
    let context = Context::default();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| show_preview(ui, &document, &appearance(), colors(), &mut Preview),
    );
    output.textures_delta.clear();
    let bounds = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .unwrap()
    };
    let baseline = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    text.galley.rows.first().and_then(|row| {
                        row.glyphs
                            .first()
                            .map(|glyph| text.pos.y + row.pos.y + glyph.pos.y)
                    })
                }
                _ => None,
            })
            .unwrap()
    };
    for (marker, label) in baseline_labels {
        assert!(
            (baseline(&marker) - baseline(label)).abs() <= TOLERANCE,
            "{marker}/{label}"
        );
    }
    let origin = bounds("origin");
    for label in ["short marker", "wide marker"] {
        assert!(
            (bounds(label).left() - origin.left() - INDENT).abs() <= TOLERANCE,
            "{label}: {:?}",
            bounds(label)
        );
    }
    let (columns, _) = table_bounds(&table_edges(&output), 1, 1);
    let center = (columns[0] + columns[1]) * CENTER_FRACTION;
    let task_icon = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.rect.size() == egui::vec2(TASK_ICON_SIZE, TASK_ICON_SIZE) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        (task_icon.union(bounds("task item")).center().x - center - INDENT * CENTER_FRACTION).abs()
            <= TOLERANCE
    );
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "\u{2022}")));
    assert!(
        (bounds("centered item").center().x - center - INDENT * CENTER_FRACTION).abs() <= TOLERANCE
    );
    let code = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "short\nlonger code line" => {
                Some(text)
            }
            _ => None,
        })
        .unwrap();
    for row in &code.galley.rows {
        assert!((row.rect().translate(code.pos.to_vec2()).center().x - center).abs() <= TOLERANCE);
    }
}

#[test]
fn 복합_표_제목의_각_문단과_줄은_표의_중앙에_정렬된다() {
    use taide_native_editor::documentation::{Alignment, TableCell, TableLayout};
    const TOLERANCE: f32 = 1.0;
    const CENTER_FRACTION: f32 = 0.5;
    const WIDTHS: [f32; 2] = [210.0, 600.0];
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![],
            rows: vec![vec![text("wide table body with several words")]],
            layout: Some(TableLayout {
                caption: vec![],
                caption_blocks: Some(vec![
                    Block::Paragraph(text("short")),
                    Block::Paragraph(vec![
                        Inline::Text(Span {
                            text: "long emphasized ".into(),
                            style: taide_native_editor::documentation::Style {
                                bold: true,
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                        Inline::Text(Span {
                            text: "caption paragraph wraps on a narrow table".into(),
                            ..Default::default()
                        }),
                    ]),
                ]),
                cells: vec![vec![TableCell {
                    column: 0,
                    column_span: 1,
                    row_span: 1,
                    alignment: Alignment::Default,
                    blocks: None,
                }]],
                column_count: 1,
                row_count: 1,
            }),
        }],
    };
    for width in WIDTHS {
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
        let (columns, rows) = table_bounds(&table_edges(&output), 1, 1);
        let center = (columns[0] + columns[1]) * CENTER_FRACTION;
        let mut paragraphs = 0;
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape
                && matches!(
                    text.galley.job.text.as_str(),
                    "short" | "long emphasized caption paragraph wraps on a narrow table"
                )
            {
                paragraphs += 1;
                for row in &text.galley.rows {
                    let rect = row.rect().translate(text.pos.to_vec2());
                    assert!(
                        (rect.center().x - center).abs() <= TOLERANCE,
                        "caption row center {} != table {center}; {}",
                        rect.center().x,
                        text.galley.job.text
                    );
                    assert!(rect.bottom() <= rows[0]);
                }
            }
        }
        assert_eq!(paragraphs, 2);
    }
}

#[test]
fn 복합_제목의_여러줄_링크는_실제_문자_영역에서만_열린다() {
    use taide_native_editor::documentation::{Link, Style, TableLayout};
    const TARGET: &str = "https://example.com/rich-caption";
    const CAPTION: &str = "강조 prefix\ncaption link tail";
    struct Links(Vec<String>);
    impl Provider for Links {
        fn open_link(&mut self, _: &Ui, target: &str) -> bool {
            self.0.push(target.into());
            true
        }
    }
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![],
            rows: vec![],
            layout: Some(TableLayout {
                caption: vec![],
                cells: vec![],
                column_count: 0,
                row_count: 0,
                caption_blocks: Some(vec![Block::Paragraph(vec![
                    Inline::Text(Span {
                        text: "강조 prefix".into(),
                        style: Style {
                            bold: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    Inline::Break,
                    Inline::Text(Span {
                        text: "caption link".into(),
                        link: Some(Link {
                            target: TARGET.into(),
                            title: "caption title".into(),
                        }),
                        ..Default::default()
                    }),
                    Inline::Text(Span {
                        text: " tail".into(),
                        ..Default::default()
                    }),
                ])]),
            }),
        }],
    };
    let mut links = Links(vec![]);
    let paint = |links: &mut Links, events| {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| show_preview(ui, &document, &appearance(), colors(), links),
        );
        output.textures_delta.clear();
        output
    };
    let output = paint(&mut links, vec![]);
    let shape = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == CAPTION => Some(text),
            _ => None,
        })
        .unwrap();
    let prefix = shape.galley.rows[0].glyphs[0]
        .logical_rect()
        .translate(shape.pos.to_vec2() + shape.galley.rows[0].pos.to_vec2())
        .center();
    let link = shape.galley.rows[1].glyphs[0]
        .logical_rect()
        .translate(shape.pos.to_vec2() + shape.galley.rows[1].pos.to_vec2())
        .center();
    let tail = shape.galley.rows[1]
        .glyphs
        .last()
        .unwrap()
        .logical_rect()
        .translate(shape.pos.to_vec2() + shape.galley.rows[1].pos.to_vec2())
        .center();
    for (position, expected) in [(prefix, 0), (tail, 0), (link, 1)] {
        for pressed in [true, false] {
            paint(
                &mut links,
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
        assert_eq!(links.0.len(), expected);
    }
    assert_eq!(links.0, [TARGET]);
    assert!(table_edges(&output).is_empty());
}

#[test]
fn 셀이_없는_표의_제목도_표시하고_링크_id를_유지한다() {
    const TARGET: &str = "https://example.com/caption";
    const CAPTION: &str = "caption without cells";
    struct Links(Vec<String>);
    impl Provider for Links {
        fn open_link(&mut self, _: &Ui, target: &str) -> bool {
            self.0.push(target.into());
            true
        }
    }
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::Table {
            columns: vec![],
            header: vec![],
            rows: vec![],
            layout: Some(taide_native_editor::documentation::TableLayout {
                caption_blocks: None,
                caption: vec![Inline::Text(Span {
                    text: CAPTION.into(),
                    link: Some(taide_native_editor::documentation::Link {
                        target: TARGET.into(),
                        title: String::new(),
                    }),
                    ..Default::default()
                })],
                cells: vec![],
                column_count: 0,
                row_count: 0,
            }),
        }],
    };
    let mut links = Links(Vec::new());
    let paint = |links: &mut Links, events| {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| show_preview(ui, &document, &appearance(), colors(), links),
        );
        output.textures_delta.clear();
        output
    };
    let output = paint(&mut links, Vec::new());
    let position = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == CAPTION => {
                Some(Rect::from_min_size(text.pos, text.galley.size()).center())
            }
            _ => None,
        })
        .unwrap();
    assert!(table_edges(&output).is_empty());
    for pressed in [true, false] {
        paint(
            &mut links,
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
    assert_eq!(links.0, [TARGET]);
}

#[test]
fn 순서_목록은_음수에서_0을_거쳐_증가하고_큰_번호에서도_overflow하지_않는다() {
    const NEGATIVE_START: i64 = -1;
    let context = Context::default();
    let document = RichDocument {
        blocks: vec![Block::List {
            style: Default::default(),
            start: Some(NEGATIVE_START),
            items: vec![
                taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: None,
                    marker_style: None,
                    blocks: vec![Block::Paragraph(text("first"))],
                },
                taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: None,
                    marker_style: None,
                    blocks: vec![Block::Paragraph(text("second"))],
                },
                taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: None,
                    marker_style: None,
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
            style: Default::default(),
            start: Some(ORDINALS[0]),
            items: ORDINALS
                .into_iter()
                .map(|ordinal| taide_native_editor::documentation::ListItem {
                    checked: None,
                    ordinal: Some(ordinal),
                    marker_style: None,
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

#[test]
fn 항목별_type은_두_표시에서_번호_도형_생략을_그린다() {
    use taide_native_editor::documentation::{ListItem, ListMarkerStyle};
    const ORDINAL: i64 = 27;
    let styles = [
        (ListMarkerStyle::UpperAlpha, Some("AA.")),
        (ListMarkerStyle::LowerRoman, Some("xxvii.")),
        (ListMarkerStyle::Circle, Some("\u{25e6}")),
        (ListMarkerStyle::Square, Some("\u{25aa}")),
        (ListMarkerStyle::None, None),
    ];
    for is_preview in [false, true] {
        for (style, marker) in styles {
            let document = RichDocument {
                blocks: vec![Block::List {
                    style: Default::default(),
                    start: None,
                    items: vec![ListItem {
                        checked: None,
                        ordinal: Some(ORDINAL),
                        marker_style: Some(style),
                        blocks: vec![Block::Paragraph(text("typed item"))],
                    }],
                }],
            };
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
            let labels = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(labels.contains(&"typed item"));
            if let Some(marker) = marker {
                assert!(labels.contains(&marker));
            } else {
                assert_eq!(labels, vec!["typed item"]);
            }
        }
    }
}
