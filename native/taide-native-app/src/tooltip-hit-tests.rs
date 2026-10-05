use super::*;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const TRIGGER: Rect = Rect::from_min_max(egui::pos2(200.0, 100.0), egui::pos2(224.0, 124.0));
const OPENED: f64 = 1.0;
const SETTLED: f64 = 2.0;
const POINTER_TIME: f64 = 2.01;
const PRESS_TIME: f64 = 2.02;
const RELEASE_TIME: f64 = 2.03;
const REGION_SIZE: egui::Vec2 = egui::vec2(80.0, 40.0);
const MOVED: egui::Pos2 = egui::pos2(120.0, 80.0);
const LEFT_POINT: egui::Pos2 = egui::pos2(10.0, 20.0);
const RIGHT_POINT: egui::Pos2 = egui::pos2(70.0, 20.0);
const REMOVED: f64 = 3.0;

#[derive(serde::Deserialize)]
struct Bounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Bounds {
    fn rect(&self) -> Rect {
        Rect::from_min_size(
            egui::pos2(self.x, self.y),
            egui::vec2(self.width, self.height),
        )
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sample {
    name: String,
    x: f32,
    y: f32,
    hit_tooltip: bool,
}

#[derive(serde::Deserialize)]
struct Case {
    side: String,
    body: Bounds,
    arrow: Bounds,
    samples: Vec<Sample>,
}

#[derive(serde::Deserialize)]
struct Source {
    results: Vec<Case>,
}

#[test]
fn layer_input_region은_이동후_최신_영역과_프레임_viewport_회수를_보존한다() {
    let context = Context::default();
    context.set_embed_viewports(false);
    assert!(context.layer_id_at(LEFT_POINT).is_none());
    let id = Id::new("owned-layer-region");
    let layer = egui::LayerId::new(egui::Order::Tooltip, id);
    let auxiliary = ViewportId(Id::new("region-auxiliary"));
    let token = Arc::new(());
    let render = |viewport, time, origin: Pos2, left, visible: bool| {
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: viewport,
                viewports: [
                    (ViewportId::ROOT, egui::ViewportInfo::default()),
                    (
                        auxiliary,
                        egui::ViewportInfo {
                            parent: Some(ViewportId::ROOT),
                            ..Default::default()
                        },
                    ),
                ]
                .into_iter()
                .collect(),
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                if viewport == ViewportId::ROOT {
                    ui.ctx().show_viewport_deferred(
                        auxiliary,
                        egui::ViewportBuilder::default(),
                        |_, _| {},
                    );
                }
                if !visible {
                    return;
                }
                let area = egui::Area::new(id)
                    .order(egui::Order::Tooltip)
                    .fixed_pos(origin)
                    .default_size(REGION_SIZE)
                    .fade_in(false)
                    .show(ui.ctx(), |ui| {
                        let (bounds, _) = ui.allocate_exact_size(REGION_SIZE, egui::Sense::CLICK);
                        let token = Arc::clone(&token);
                        ui.ctx()
                            .set_layer_input_region(layer, bounds, move |point| {
                                let _owned = &token;
                                if left {
                                    point.x < bounds.center().x
                                } else {
                                    point.x >= bounds.center().x
                                }
                            });
                        ui.is_sizing_pass()
                    });
                assert_eq!(
                    ui.ctx().layer_id_at(origin + LEFT_POINT.to_vec2()) == Some(layer),
                    left && !area.inner,
                    "viewport={viewport:?} sizing={} origin={origin:?}",
                    area.inner
                );
                assert_eq!(
                    ui.ctx().layer_id_at(origin + RIGHT_POINT.to_vec2()) == Some(layer),
                    !left && !area.inner
                );
            },
        );
        output.textures_delta.clear();
    };
    render(ViewportId::ROOT, OPENED, Pos2::ZERO, true, true);
    render(ViewportId::ROOT, OPENED, Pos2::ZERO, true, true);
    assert_eq!(context.layer_id_at(LEFT_POINT), Some(layer));
    render(auxiliary, OPENED, Pos2::ZERO, false, true);
    render(auxiliary, OPENED, Pos2::ZERO, false, true);
    assert_eq!(Arc::strong_count(&token), 3);
    render(ViewportId::ROOT, SETTLED, MOVED, true, true);
    assert_eq!(
        context.layer_id_at(MOVED + LEFT_POINT.to_vec2()),
        Some(layer)
    );
    assert_ne!(context.layer_id_at(LEFT_POINT), Some(layer));
    assert_eq!(Arc::strong_count(&token), 3);
    render(ViewportId::ROOT, REMOVED, MOVED, true, false);
    assert_eq!(Arc::strong_count(&token), 2);
    render(auxiliary, REMOVED, Pos2::ZERO, false, false);
    assert_eq!(Arc::strong_count(&token), 1);
    assert_ne!(
        context.layer_id_at(MOVED + LEFT_POINT.to_vec2()),
        Some(layer)
    );
}

#[test]
fn tooltip_hit은_source16위치의_rounded_arrow와_뒤_button_투과를_보존한다() {
    let source: Source = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-04-tooltip-hit-source.json"
    ))
    .unwrap();
    for case in source.results {
        let align = match case.side.as_str() {
            "top" => egui::RectAlign::TOP,
            "bottom" => egui::RectAlign::BOTTOM,
            "left" => egui::RectAlign::LEFT,
            "right" => egui::RectAlign::RIGHT,
            _ => panic!("unexpected source side"),
        };
        for sample in case.samples {
            let context = Context::default();
            let provider = Provider::default();
            let appearance = Appearance {
                background: Color32::BLACK,
                border: Color32::GRAY,
                foreground: Color32::WHITE,
            };
            let id = Id::new("source-tooltip-hit");
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
                                Id::new("source-tooltip-underlay"),
                                egui::Sense::click(),
                            )
                            .clicked();
                        let response = ui.interact(TRIGGER, id, egui::Sense::click());
                        provider.show(&response, "Tooltip reference", align, &appearance);
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
            render(SETTLED, Vec::new());
            let (body, arrow) = {
                let viewports = provider.0.lock().unwrap();
                let widget = viewports
                    .get(&ViewportId::ROOT)
                    .unwrap()
                    .widgets
                    .get(&id)
                    .unwrap();
                (widget.content.unwrap(), widget.arrow.unwrap())
            };
            let expected = egui::pos2(sample.x, sample.y);
            let point = if sample.name.starts_with("arrow-") {
                arrow.center + (expected - case.arrow.rect().center())
            } else {
                body.min + (expected - case.body.rect().min)
            };
            let layer = egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Tooltip::next_tooltip_id(&context, id),
            );
            assert_eq!(
                context.layer_id_at(point) == Some(layer),
                sample.hit_tooltip,
                "{} {} at {point:?}",
                case.side,
                sample.name
            );
            render(POINTER_TIME, vec![Event::PointerMoved(point)]);
            let press = |pressed| Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            assert!(!render(PRESS_TIME, vec![press(true)]));
            assert_eq!(
                render(RELEASE_TIME, vec![press(false)]),
                !sample.hit_tooltip,
                "{} {} actual Button",
                case.side,
                sample.name
            );
        }
    }
}
