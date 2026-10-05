use super::*;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const TRIGGER: Rect = Rect::from_min_max(egui::pos2(200.0, 100.0), egui::pos2(224.0, 124.0));
const CONTENT_HEIGHT: f32 = 30.0;
const SOURCE_GAP: f32 = 10.0;
const SOURCE_LINE_HEIGHT: f32 = 16.0;
const FONT: f32 = 12.0;
const SOURCE_ARROW_RADIUS: u8 = 2;
const FINAL: f64 = 1.0;
const DURING_FADE: f64 = 0.03;
const SUBPIXEL_TOLERANCE: f32 = 0.03125;
const ARROW_TIP_DISTANCE: f32 = 6.0;
const ARROW_EMPTY_CORNER: f32 = 5.0;
const AFTER_TIP: f64 = 1.01;
const AT_TIP_CLICK: f64 = 1.02;
const AT_EMPTY_CLICK: f64 = 1.03;
const CHANGED_LABEL: &str = "Tooltip reference Tooltip reference";
const BEFORE_LABEL_CHANGE: f64 = 2.0;
const AFTER_LABEL_CHANGE: f64 = 3.0;

#[test]
fn tooltip_geometry는_라벨_변경후_재배치를_예약한다() {
    let context = Context::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let render = |time, label| {
        let mut bounds = None;
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                let response = ui.interact(
                    TRIGGER,
                    Id::new("tooltip-label-change"),
                    egui::Sense::hover(),
                );
                bounds = appearance.show_controlled(&response, label, egui::RectAlign::TOP);
            },
        );
        output.textures_delta.clear();
        (output, bounds.unwrap())
    };
    render(0.0, "Tooltip reference");
    render(0.0, "Tooltip reference");
    render(FINAL, "Tooltip reference");
    render(BEFORE_LABEL_CHANGE, "Tooltip reference");
    let (changed, _) = render(AFTER_LABEL_CHANGE, CHANGED_LABEL);
    assert_eq!(
        changed
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .unwrap()
            .repaint_delay,
        std::time::Duration::ZERO,
    );
    let (_, bounds) = render(AFTER_LABEL_CHANGE, CHANGED_LABEL);
    assert!((bounds.center().x - TRIGGER.center().x).abs() < 1.0);
    assert_eq!(TRIGGER.top() - bounds.bottom(), SOURCE_GAP);
}

#[derive(serde::Deserialize)]
struct SourceBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl SourceBounds {
    fn rect(&self) -> Rect {
        Rect::from_min_size(
            egui::pos2(self.x, self.y),
            egui::vec2(self.width, self.height),
        )
    }
}

#[derive(serde::Deserialize)]
struct SourceSample {
    id: String,
    side: String,
    trigger: SourceBounds,
    content: SourceBounds,
    arrow: SourceBounds,
}

#[derive(serde::Deserialize)]
struct SourceMeasurement {
    samples: Vec<SourceSample>,
}

#[test]
fn tooltip_arrow는_원본_dom_8개_배치와_flip_shift를_보존한다() {
    let source: SourceMeasurement = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-04-tooltip-source.json"
    ))
    .unwrap();
    let side = |side: &str| match side {
        "top" => egui::RectAlign::TOP,
        "bottom" => egui::RectAlign::BOTTOM,
        "left" => egui::RectAlign::LEFT,
        "right" => egui::RectAlign::RIGHT,
        _ => panic!("unexpected source side {side}"),
    };
    for sample in source.samples {
        let requested = match sample.id.as_str() {
            "flip-top" => egui::RectAlign::TOP,
            "flip-bottom" => egui::RectAlign::BOTTOM,
            _ => side(&sample.side),
        };
        let placement = Placement::new(
            sample.trigger.rect(),
            sample.content.rect().size(),
            Rect::from_min_size(Pos2::ZERO, SCREEN),
            requested,
            1.0,
        );
        assert_eq!(placement.align, side(&sample.side), "{}", sample.id);
        assert!(
            placement.position.distance(sample.content.rect().min) < SUBPIXEL_TOLERANCE,
            "{}: {:?} != {:?}",
            sample.id,
            placement.position,
            sample.content.rect().min,
        );
        let arrow = placement
            .arrow(sample.trigger.rect(), sample.content.rect())
            .unwrap();
        assert!(
            arrow.center.distance(sample.arrow.rect().center()) < SUBPIXEL_TOLERANCE,
            "{}: {:?} != {:?}",
            sample.id,
            arrow.center,
            sample.arrow.rect().center(),
        );
    }
}

#[test]
fn tooltip_arrow는_첫_sizing과_fade에서_본문의_표시상태를_공유한다() {
    let context = Context::default();
    let appearance = Appearance {
        background: Color32::from_rgb(24, 24, 37),
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let render = |time| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                let response = ui.interact(TRIGGER, Id::new("tooltip-fade"), egui::Sense::hover());
                appearance.show_controlled(&response, "Tooltip reference", egui::RectAlign::TOP);
            },
        );
        output.textures_delta.clear();
        output
    };
    let first = render(0.0);
    assert!(!first.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Rect(rect) if rect.angle != 0.0 && rect.fill.a() > 0
    )));
    render(0.0);
    let fading = render(DURING_FADE);
    let rectangles = fading.shapes.iter().filter_map(|shape| match &shape.shape {
        egui::Shape::Rect(rect) if rect.fill.a() > 0 => Some(rect),
        _ => None,
    });
    let body = rectangles
        .clone()
        .find(|rect| rect.angle == 0.0 && !rect.stroke.is_empty())
        .unwrap();
    let arrow = rectangles
        .into_iter()
        .find(|rect| rect.angle != 0.0)
        .unwrap();
    assert!(body.fill.a() < appearance.background.a());
    assert_eq!(arrow.fill, body.fill);
}

#[test]
fn tooltip_arrow는_실제_잉크의_hover_click만_본문과_공유한다() {
    let context = Context::default();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("tooltip-arrow-hit");
    let render = |time, events, visible| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                if visible {
                    let response = ui.interact(TRIGGER, id, egui::Sense::click());
                    provider.show(
                        &response,
                        "Tooltip reference",
                        egui::RectAlign::TOP,
                        &appearance,
                    );
                }
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        output
    };
    render(0.0, Vec::new(), true);
    render(0.0, vec![Event::PointerMoved(TRIGGER.center())], true);
    render(DELAY_SECONDS, Vec::new(), true);
    render(DELAY_SECONDS, Vec::new(), true);
    render(FINAL, Vec::new(), true);
    let (bounds, arrow) = {
        let viewports = provider.0.lock().unwrap();
        let viewport = viewports.get(&egui::ViewportId::ROOT).unwrap();
        assert_eq!(viewport.open, Some(id));
        let widget = viewport.widgets.get(&id).unwrap();
        (widget.content.unwrap(), widget.arrow.unwrap())
    };
    let tip = arrow.center + egui::vec2(0.0, ARROW_TIP_DISTANCE);
    let corner = arrow.center + egui::Vec2::splat(ARROW_EMPTY_CORNER);
    assert!(!bounds.contains(tip));
    assert!(arrow.contains(tip));
    assert!(!bounds.contains(corner));
    assert!(!arrow.contains(corner));
    render(AFTER_TIP, vec![Event::PointerMoved(tip)], true);
    let click = |pos| Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    };
    render(AT_TIP_CLICK, vec![click(tip)], true);
    assert_eq!(
        provider
            .0
            .lock()
            .unwrap()
            .get(&egui::ViewportId::ROOT)
            .unwrap()
            .open,
        Some(id)
    );
    render(
        AT_EMPTY_CLICK,
        vec![Event::PointerMoved(corner), click(corner)],
        true,
    );
    assert_eq!(
        provider
            .0
            .lock()
            .unwrap()
            .get(&egui::ViewportId::ROOT)
            .unwrap()
            .open,
        None
    );
    render(FINAL + FINAL, Vec::new(), false);
    assert!(
        provider
            .0
            .lock()
            .unwrap()
            .get(&egui::ViewportId::ROOT)
            .unwrap()
            .widgets
            .is_empty()
    );
}

#[test]
fn tooltip_원본_dom은_16px_line과_10px_gap_arrow를_실제_renderer에_보존한다() {
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-tooltip-placement-{}", uuid::Uuid::new_v4())),
    ));
    let theme = taide_runtime::theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
    let appearance = Appearance::new(&theme).unwrap();
    for align in [
        egui::RectAlign::TOP,
        egui::RectAlign::BOTTOM,
        egui::RectAlign::LEFT,
        egui::RectAlign::RIGHT,
    ] {
        let context = Context::default();
        context.enable_accesskit();
        let render = |time| {
            let mut bounds = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    ..Default::default()
                },
                |ui| {
                    let response =
                        ui.interact(TRIGGER, Id::new("tooltip-placement"), egui::Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "trigger")
                    });
                    bounds = appearance.show_controlled(&response, "Tooltip reference", align);
                },
            );
            output.textures_delta.clear();
            (output, bounds.unwrap())
        };
        render(0.0);
        render(0.0);
        let (output, bounds) = render(FINAL);
        assert_eq!(bounds.height(), CONTENT_HEIGHT);
        let gap = if align == egui::RectAlign::TOP {
            TRIGGER.top() - bounds.bottom()
        } else if align == egui::RectAlign::BOTTOM {
            bounds.top() - TRIGGER.bottom()
        } else if align == egui::RectAlign::LEFT {
            TRIGGER.left() - bounds.right()
        } else {
            bounds.left() - TRIGGER.right()
        };
        assert!((gap - SOURCE_GAP).abs() < 1.0);
        let text = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Tooltip reference" => Some(text),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            text.galley.job.sections[0].format.font_id,
            egui::FontId::proportional(FONT)
        );
        assert_eq!(
            text.galley.job.sections[0].format.line_height,
            Some(SOURCE_LINE_HEIGHT)
        );
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.stroke.is_empty() && rect.fill == appearance.background
                && rect.rect.size() == egui::Vec2::splat(SOURCE_GAP)
                && rect.angle == std::f32::consts::FRAC_PI_4
                && rect.corner_radius == egui::CornerRadius::same(SOURCE_ARROW_RADIUS)
        )));
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let (tooltip_id, tooltip) = nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
            .unwrap();
        assert_eq!(tooltip.label(), Some("Tooltip reference"));
        assert!(!tooltip.supports_action(egui::accesskit::Action::Focus));
        assert_eq!(
            nodes
                .iter()
                .find(|(id, _)| *id == Id::new("tooltip-placement").accesskit_id())
                .unwrap()
                .1
                .described_by(),
            [*tooltip_id]
        );
    }
}
