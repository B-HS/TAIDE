use egui::{Color32, Context, FontId, RawInput, Rect, Ui, Vec2};
use taide_native_editor::documentation::{Block, Inline, RichDocument, Span};
use taide_native_ui::editor_markup::{Colors, Provider, show_preview};
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
    assert!(output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.stroke.width == CELL_BORDER_WIDTH && rect.stroke.color == colors().border)).count() >= TABLE_CELL_COUNT);
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
