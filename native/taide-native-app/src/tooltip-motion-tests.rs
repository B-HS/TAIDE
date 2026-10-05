use super::*;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const TRIGGER: Rect = Rect::from_min_max(egui::pos2(200.0, 100.0), egui::pos2(224.0, 124.0));
const OPENED: f64 = 1.0;
const SETTLED: f64 = 2.0;
const CLOSED: f64 = 3.0;
const DURATION: f64 = 0.15;
const MILLISECONDS: f64 = 1000.0;
const VALUE_TOLERANCE: f32 = 0.00001;
const RECT_TOLERANCE: f32 = 0.001;
const FADE_TIME: f64 = 0.075;
const HIT_TIME: f64 = 0.03;
const HIT_INSET: f32 = 1.0;
const CHILD_OFFSET: egui::Vec2 = egui::vec2(80.0, 40.0);
const FOCUS_STEP: f64 = 0.032;
const SECOND_TRIGGER: Rect = Rect::from_min_max(egui::pos2(360.0, 100.0), egui::pos2(384.0, 124.0));

#[test]
fn tooltip_motion은_source의_닫힘_presence_중_focus_재진입을_다시_닫는다() {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-04-tooltip-events-source.json"
    ))
    .unwrap();
    let cases = source["results"].as_array().unwrap();
    let plain = cases.iter().find(|case| case["mode"] == "plain").unwrap();
    let samples = plain["samples"].as_array().unwrap();
    let expected = [Some("first"), Some("second"), None];
    for (sample, expected) in samples.iter().skip(1).take(expected.len()).zip(expected) {
        let open = sample["contents"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|content| content["state"] != "closed")
            .map(|content| content["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(open, expected.into_iter().collect::<Vec<_>>());
    }
    let context = Context::default();
    context.enable_accesskit();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let ids = [
        Id::new("focus-presence-first"),
        Id::new("focus-presence-second"),
    ];
    let render = |time| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                for (id, rect) in ids.into_iter().zip([TRIGGER, SECOND_TRIGGER]) {
                    let response = ui.interact(rect, id, egui::Sense::click());
                    provider.show(
                        &response,
                        "Focus reference",
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
    render(0.0);
    let mut now = OPENED;
    for (id, expected) in
        [ids[0], ids[1], ids[0]]
            .into_iter()
            .zip([Some(ids[0]), Some(ids[1]), None])
    {
        context.memory_mut(|memory| memory.request_focus(id));
        let output = render(now);
        let owners = provider.0.lock().unwrap();
        let viewport = owners.get(&ViewportId::ROOT).unwrap();
        assert_eq!(viewport.open, expected, "focus={id:?} time={now}");
        for id in ids {
            let node = output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(node_id, _)| *node_id == id.accesskit_id())
                .unwrap();
            assert_eq!(!node.1.described_by().is_empty(), expected == Some(id));
        }
        drop(owners);
        now += FOCUS_STEP;
    }
    context.memory_mut(|memory| memory.surrender_focus(ids[0]));
    render(SETTLED);
    context.memory_mut(|memory| memory.request_focus(ids[0]));
    render(SETTLED + FOCUS_STEP);
    assert_eq!(
        provider.0.lock().unwrap()[&ViewportId::ROOT].open,
        Some(ids[0]),
        "expired Content must not receive a new global open event"
    );
}

#[derive(serde::Deserialize)]
struct MotionBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl MotionBounds {
    fn rect(&self) -> Rect {
        Rect::from_min_size(
            egui::pos2(self.x, self.y),
            egui::vec2(self.width, self.height),
        )
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MotionSample {
    time: f64,
    opacity: f32,
    scale: f32,
    translate_x: f32,
    translate_y: f32,
    origin: [f32; 2],
    content: MotionBounds,
}

#[derive(serde::Deserialize)]
struct MotionCase {
    side: String,
    phase: String,
    duration: f64,
    base: MotionBounds,
    samples: Vec<MotionSample>,
}

#[derive(serde::Deserialize)]
struct MotionSource {
    results: Vec<MotionCase>,
}

#[test]
fn tooltip_motion은_이동한_본문의_실제_뒤쪽_button_click을_차단한다() {
    let context = Context::default();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("tooltip-moving-hit");
    let render = |time, events| {
        let mut clicked = false;
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                clicked = ui
                    .interact(
                        Rect::from_min_size(Pos2::ZERO, SCREEN),
                        Id::new("behind-tooltip"),
                        egui::Sense::click(),
                    )
                    .clicked();
                let response = ui.interact(TRIGGER, id, egui::Sense::click());
                provider.show(
                    &response,
                    "Tooltip reference",
                    egui::RectAlign::TOP,
                    &appearance,
                );
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        clicked
    };
    render(0.0, Vec::new());
    context.memory_mut(|memory| memory.request_focus(id));
    render(OPENED, Vec::new());
    render(OPENED, Vec::new());
    render(OPENED + HIT_TIME, Vec::new());
    let tooltip = egui::Tooltip::next_tooltip_id(&context, id);
    let base = egui::AreaState::load(&context, tooltip).unwrap().rect();
    let painted = provider
        .0
        .lock()
        .unwrap()
        .get(&ViewportId::ROOT)
        .unwrap()
        .widgets
        .get(&id)
        .unwrap()
        .content
        .unwrap();
    let point = egui::pos2(painted.center().x, painted.bottom() - HIT_INSET);
    assert!(
        !base.contains(point),
        "sample must expose animation-only pixels"
    );
    assert_eq!(
        context.layer_id_at(point),
        Some(egui::LayerId::new(egui::Order::Tooltip, tooltip))
    );
    render(OPENED + HIT_TIME, vec![Event::PointerMoved(point)]);
    let press = |pressed| Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    assert!(!render(OPENED + HIT_TIME, vec![press(true)]));
    assert!(!render(OPENED + HIT_TIME, vec![press(false)]));
}

#[test]
fn tooltip_motion은_화면_제거시_남은_layer_transform을_회수한다() {
    let context = Context::default();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("tooltip-unmounted-transform");
    let render = |time, visible| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
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
    };
    render(0.0, true);
    context.memory_mut(|memory| memory.request_focus(id));
    render(OPENED, true);
    render(OPENED + HIT_TIME, true);
    let layer = egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Tooltip::next_tooltip_id(&context, id),
    );
    assert!(context.layer_transform_to_global(layer).is_some());
    render(OPENED + FADE_TIME, false);
    assert!(context.layer_transform_to_global(layer).is_none());
    assert!(
        provider
            .0
            .lock()
            .unwrap()
            .get(&ViewportId::ROOT)
            .unwrap()
            .widgets
            .is_empty()
    );
}

#[test]
fn tooltip_motion은_controlled_오류_제거시_transform을_회수하고_escape는_소비하지_않는다() {
    let context = Context::default();
    context.enable_accesskit();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("controlled-tooltip-transform");
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Tooltip::tooltip_id(id, 0));
    let render = |time, visible: bool, events| {
        let mut escape = false;
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                let response = ui.interact(TRIGGER, id, egui::Sense::click());
                if visible {
                    provider.show_controlled(
                        &response,
                        Some("Validation error"),
                        egui::RectAlign::BOTTOM,
                        &appearance,
                    );
                }
                escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        (output, escape)
    };
    context.memory_mut(|memory| memory.request_focus(id));
    render(OPENED, true, Vec::new());
    render(OPENED + HIT_TIME, true, Vec::new());
    assert!(context.layer_transform_to_global(layer).is_some());
    let (_, escape) = render(
        OPENED + HIT_TIME,
        true,
        vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(
        escape,
        "controlled validation must leave Escape to its Explorer owner"
    );
    let (removed, _) = render(OPENED + FADE_TIME, false, Vec::new());
    assert!(context.layer_transform_to_global(layer).is_none());
    assert!(
        !removed
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
    );
}

#[test]
fn tooltip_motion은_제거된_viewport의_layer만_회수하고_살아있는_layer를_유지한다() {
    viewport_layer_cleanup(false);
}

#[test]
fn tooltip_motion은_같은_widget_id의_두_viewport_transform을_격리한다() {
    viewport_layer_cleanup(true);
}

fn viewport_layer_cleanup(same_id: bool) {
    let context = Context::default();
    context.set_embed_viewports(false);
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let child = ViewportId::from_hash_of("tooltip-transform-child");
    let root_id = Id::new("tooltip-root-transform");
    let child_id = if same_id {
        root_id
    } else {
        Id::new("tooltip-child-transform")
    };
    let root_layer =
        egui::LayerId::new(egui::Order::Tooltip, egui::Tooltip::tooltip_id(root_id, 0));
    let child_layer = egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Tooltip::tooltip_id(child_id.with(child), 0),
    );
    let render = |viewport, time, live: bool| {
        let mut viewports = [(ViewportId::ROOT, egui::ViewportInfo::default())]
            .into_iter()
            .collect::<egui::ViewportIdMap<_>>();
        if live {
            viewports.insert(
                child,
                egui::ViewportInfo {
                    parent: Some(ViewportId::ROOT),
                    ..Default::default()
                },
            );
        }
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: viewport,
                viewports,
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                if viewport == ViewportId::ROOT && live {
                    ui.ctx().show_viewport_deferred(
                        child,
                        egui::ViewportBuilder::default(),
                        |_, _| {},
                    );
                }
                provider.begin_frame(ui.ctx());
                let id = if viewport == ViewportId::ROOT {
                    root_id
                } else {
                    child_id
                };
                let trigger = if viewport == ViewportId::ROOT {
                    TRIGGER
                } else {
                    TRIGGER.translate(CHILD_OFFSET)
                };
                let response = ui.interact(trigger, id, egui::Sense::click());
                provider.show_controlled(
                    &response,
                    Some("Validation error"),
                    egui::RectAlign::BOTTOM,
                    &appearance,
                );
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
    };
    render(ViewportId::ROOT, OPENED, true);
    render(child, OPENED, true);
    render(ViewportId::ROOT, OPENED + HIT_TIME, true);
    let root_transform = context.layer_transform_to_global(root_layer).unwrap();
    render(child, OPENED + HIT_TIME, true);
    assert_eq!(
        context.layer_transform_to_global(root_layer),
        Some(root_transform)
    );
    assert_ne!(
        context.layer_transform_to_global(child_layer).unwrap(),
        root_transform
    );
    render(ViewportId::ROOT, OPENED + FADE_TIME, false);
    assert!(context.layer_transform_to_global(root_layer).is_some());
    assert!(context.layer_transform_to_global(child_layer).is_none());
    let viewports = provider.0.lock().unwrap();
    assert_eq!(viewports.len(), 1);
    assert!(!viewports.contains_key(&child));
}

#[test]
fn tooltip_motion은_disabled_및_context_교체에서_이전_transform을_회수한다() {
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("tooltip-disabled-context-transform");
    let render = |context: &Context, time, enabled| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                let response = ui
                    .add_enabled_ui(enabled, |ui| ui.interact(TRIGGER, id, egui::Sense::click()))
                    .inner;
                provider.show(
                    &response,
                    "Tooltip reference",
                    egui::RectAlign::TOP,
                    &appearance,
                );
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
    };
    let context = Context::default();
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Tooltip::tooltip_id(id, 0));
    render(&context, 0.0, true);
    context.memory_mut(|memory| memory.request_focus(id));
    render(&context, OPENED, true);
    render(&context, OPENED + HIT_TIME, true);
    assert!(context.layer_transform_to_global(layer).is_some());
    render(&context, OPENED + FADE_TIME, false);
    assert!(
        context.layer_transform_to_global(layer).is_none(),
        "disabled must release its transform"
    );
    context.memory_mut(|memory| memory.request_focus(id));
    render(&context, SETTLED, true);
    render(&context, SETTLED + HIT_TIME, true);
    assert!(context.layer_transform_to_global(layer).is_some());
    let replacement = Context::default();
    render(&replacement, 0.0, true);
    assert!(
        context.layer_transform_to_global(layer).is_none(),
        "Context replacement must release old graphics state"
    );
    assert!(replacement.layer_transform_to_global(layer).is_none());
}

#[test]
fn tooltip_motion은_실제_source_32개_시점의_ease_scale_slide_origin을_보존한다() {
    let source: MotionSource = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-04-tooltip-motion-source.json"
    ))
    .unwrap();
    for case in source.results {
        assert_eq!(case.duration / MILLISECONDS, DURATION);
        let side = match case.side.as_str() {
            "top" => egui::RectAlign::TOP,
            "bottom" => egui::RectAlign::BOTTOM,
            "left" => egui::RectAlign::LEFT,
            "right" => egui::RectAlign::RIGHT,
            _ => panic!("unexpected source side"),
        };
        let motion = Motion::new(case.phase == "open", 0.0);
        for expected in case.samples {
            let sample = motion.sample(expected.time / MILLISECONDS, side);
            assert!((sample.opacity - expected.opacity).abs() < VALUE_TOLERANCE);
            assert!((sample.scale - expected.scale).abs() < VALUE_TOLERANCE);
            assert!(
                (sample.translate - egui::vec2(expected.translate_x, expected.translate_y))
                    .length()
                    < VALUE_TOLERANCE
            );
            let transform = sample.transform(
                case.base.rect().min + egui::vec2(expected.origin[0], expected.origin[1]),
            );
            let bounds = transform * case.base.rect();
            assert!(bounds.min.distance(expected.content.rect().min) < RECT_TOLERANCE);
            assert!(bounds.max.distance(expected.content.rect().max) < RECT_TOLERANCE);
        }
    }
}

#[test]
fn tooltip_motion은_닫힘_150ms동안_role을_보존하고_trigger_설명은_즉시_해제한다() {
    let context = Context::default();
    context.enable_accesskit();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("tooltip-presence");
    let render = |time, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                let response = ui.interact(TRIGGER, id, egui::Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "trigger")
                });
                provider.show(
                    &response,
                    "Tooltip reference",
                    egui::RectAlign::TOP,
                    &appearance,
                );
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        output
    };
    render(0.0, Vec::new());
    context.memory_mut(|memory| memory.request_focus(id));
    render(OPENED, Vec::new());
    render(OPENED, Vec::new());
    render(SETTLED, Vec::new());
    let closed = render(
        CLOSED,
        vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    let nodes = &closed
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    assert!(
        nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
    );
    assert!(
        nodes
            .iter()
            .find(|(node, _)| *node == id.accesskit_id())
            .unwrap()
            .1
            .described_by()
            .is_empty()
    );
    let fading = render(CLOSED + FADE_TIME, Vec::new());
    let frame = fading
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.angle == 0.0 && !rect.stroke.is_empty() => Some(rect),
            _ => None,
        })
        .unwrap();
    let expected = Motion::new(false, CLOSED).sample(CLOSED + FADE_TIME, egui::RectAlign::TOP);
    assert!((f32::from(frame.fill.a()) - expected.opacity * f32::from(u8::MAX)).abs() <= 1.0);
    let text = fading
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "Tooltip reference" => Some(text),
            _ => None,
        })
        .unwrap();
    let text_bounds = Rect::from_min_size(text.pos, text.galley.size());
    let label = &fading
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == egui::accesskit::Role::Label && node.value() == Some("Tooltip reference")
        })
        .unwrap()
        .1;
    let label = label.bounds().unwrap();
    assert!(
        (label.x0 - f64::from(text_bounds.left())).abs() < f64::from(RECT_TOLERANCE),
        "AX Label bounds must follow painted motion: {label:?}, {text_bounds:?}"
    );
    assert!((label.y0 - f64::from(text_bounds.top())).abs() < f64::from(RECT_TOLERANCE));
    assert!((label.width() - f64::from(text_bounds.width())).abs() < f64::from(RECT_TOLERANCE));
    assert!((label.height() - f64::from(text_bounds.height())).abs() < f64::from(RECT_TOLERANCE));
    let removed = render(CLOSED + DURATION, Vec::new());
    assert!(
        !removed
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
    );
}
