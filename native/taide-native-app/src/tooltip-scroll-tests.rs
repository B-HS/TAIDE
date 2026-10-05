use super::*;

const SCREEN: egui::Vec2 = egui::vec2(800.0, 640.0);
const TRIGGER_SIZE: egui::Vec2 = egui::vec2(80.0, 24.0);
const AREA_WIDTH: f32 = 220.0;
const INNER_HEIGHT: f32 = 120.0;
const OUTER_HEIGHT: f32 = 280.0;
const CONTENT_HEIGHT: f32 = 400.0;
const SCROLLED: f32 = 12.0;
const OPENED: f64 = 1.0;
const INPUT_TIME: f64 = 1.1;

#[test]
fn tooltip_scroll은_실제_조상만_닫고_controlled_prop과_무변화를_보존한다() {
    let mut failures = Vec::new();
    for (case, offsets, should_close) in [
        ("inner", [0.0, SCROLLED, 0.0, 0.0], true),
        ("outer", [SCROLLED, 0.0, 0.0, 0.0], true),
        ("sibling", [0.0, 0.0, SCROLLED, 0.0], false),
        ("descendant", [0.0, 0.0, 0.0, SCROLLED], false),
        ("unchanged", [0.0; 4], false),
        ("clamped", [-SCROLLED, -SCROLLED, 0.0, 0.0], false),
    ] {
        for controlled in [None, Some(true), Some(false)] {
            for accesskit in [false, true] {
                let context = Context::default();
                if accesskit {
                    context.enable_accesskit();
                }
                let provider = Provider::default();
                let id = Id::new("scroll-trigger");
                let appearance = Appearance {
                    background: Color32::BLACK,
                    border: Color32::GRAY,
                    foreground: Color32::WHITE,
                };
                let render = |time, offsets: [f32; 4]| {
                    let mut actual_offsets = [0.0; 4];
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(time),
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                            ..Default::default()
                        },
                        |ui| {
                            provider.begin_frame(ui.ctx());
                            ui.horizontal_top(|ui| {
                                let outer = egui::ScrollArea::vertical()
                                    .id_salt("outer")
                                    .max_width(AREA_WIDTH)
                                    .max_height(OUTER_HEIGHT)
                                    .vertical_scroll_offset(offsets[0])
                                    .show(ui, |ui| {
                                        let inner = egui::ScrollArea::vertical()
                                            .id_salt("inner")
                                            .max_height(INNER_HEIGHT)
                                            .vertical_scroll_offset(offsets[1])
                                            .show(ui, |ui| {
                                                ui.scope_builder(egui::UiBuilder::new(), |ui| {
                                                    let (rect, _) = ui.allocate_exact_size(
                                                        TRIGGER_SIZE,
                                                        egui::Sense::hover(),
                                                    );
                                                    let response =
                                                        ui.interact(rect, id, egui::Sense::click());
                                                    response.widget_info(|| {
                                                        egui::WidgetInfo::labeled(
                                                            egui::WidgetType::Button,
                                                            true,
                                                            "Scroll reference",
                                                        )
                                                    });
                                                    if let Some(open) = controlled {
                                                        provider.show_controlled(
                                                            &response,
                                                            open.then_some("Scroll error"),
                                                            egui::RectAlign::TOP,
                                                            &appearance,
                                                        );
                                                    } else {
                                                        provider.show(
                                                            &response,
                                                            "Scroll reference",
                                                            egui::RectAlign::TOP,
                                                            &appearance,
                                                        );
                                                    }
                                                    let descendant = egui::ScrollArea::vertical()
                                                        .id_salt("descendant")
                                                        .max_height(INNER_HEIGHT)
                                                        .vertical_scroll_offset(offsets[3])
                                                        .show(ui, |ui| {
                                                            ui.allocate_space(egui::vec2(
                                                                AREA_WIDTH,
                                                                CONTENT_HEIGHT,
                                                            ));
                                                        });
                                                    actual_offsets[3] = descendant.state.offset.y;
                                                });
                                                ui.allocate_space(egui::vec2(
                                                    AREA_WIDTH,
                                                    CONTENT_HEIGHT,
                                                ));
                                            });
                                        actual_offsets[1] = inner.state.offset.y;
                                        ui.allocate_space(egui::vec2(AREA_WIDTH, CONTENT_HEIGHT));
                                    });
                                actual_offsets[0] = outer.state.offset.y;
                                let sibling = egui::ScrollArea::vertical()
                                    .id_salt("sibling")
                                    .max_width(AREA_WIDTH)
                                    .max_height(OUTER_HEIGHT)
                                    .vertical_scroll_offset(offsets[2])
                                    .show(ui, |ui| {
                                        ui.allocate_space(egui::vec2(AREA_WIDTH, CONTENT_HEIGHT));
                                    });
                                actual_offsets[2] = sibling.state.offset.y;
                            });
                            provider.finish_frame(ui.ctx());
                        },
                    );
                    output.textures_delta.clear();
                    (output, actual_offsets)
                };
                render(0.0, [0.0; 4]);
                if controlled.is_none() {
                    context.memory_mut(|memory| memory.request_focus(id));
                }
                render(OPENED, [0.0; 4]);
                let (output, actual_offsets) = render(INPUT_TIME, offsets);
                assert_eq!(
                    actual_offsets,
                    offsets.map(|offset| offset.max(0.0)),
                    "fixture {case}"
                );
                let owners = provider.0.lock().unwrap();
                let viewport = owners.get(&ViewportId::ROOT).unwrap();
                let expected_open = (controlled.is_none() && !should_close).then_some(id);
                let expected_skip = (should_close && controlled != Some(false))
                    .then_some(INPUT_TIME + SKIP_DELAY_SECONDS);
                if viewport.open != expected_open || viewport.skip_until != expected_skip {
                    failures.push(format!("case={case} controlled={controlled:?} accesskit={accesskit} open={:?} expected={expected_open:?} skip={:?} expected_skip={expected_skip:?}", viewport.open, viewport.skip_until));
                }
                if accesskit {
                    let node = output
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .find(|(node_id, _)| *node_id == id.accesskit_id())
                        .unwrap();
                    let expected_description = controlled.unwrap_or(!should_close);
                    if !node.1.described_by().is_empty() != expected_description {
                        failures.push(format!("case={case} controlled={controlled:?} stale AX"));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn scroll_조상은_ax_재배치와_무관하며_viewport와_pass_재배치로_회수된다() {
    for accesskit in [false, true] {
        let context = Context::default();
        context.set_embed_viewports(false);
        if accesskit {
            context.enable_accesskit();
        }
        let auxiliary = ViewportId(Id::new("scroll-auxiliary"));
        let trigger = Id::new("shared-scroll-trigger");
        let render = |viewport, time, offset, location| {
            let mut was_scrolled = false;
            let mut actual_offset = 0.0;
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
                    let scroll = egui::ScrollArea::vertical()
                        .id_salt(("physical-scroll", viewport))
                        .max_height(INNER_HEIGHT)
                        .vertical_scroll_offset(offset)
                        .show(ui, |ui| {
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .accessibility_parent(Id::new("outside-ax-parent")),
                                |ui| {
                                    if location == "inside" {
                                        let (rect, _) = ui.allocate_exact_size(
                                            TRIGGER_SIZE,
                                            egui::Sense::hover(),
                                        );
                                        ui.interact(rect, trigger, egui::Sense::click());
                                    }
                                    ui.allocate_space(egui::vec2(AREA_WIDTH, CONTENT_HEIGHT));
                                },
                            );
                        });
                    actual_offset = scroll.state.offset.y;
                    if location == "outside" {
                        let (rect, _) = ui.allocate_exact_size(TRIGGER_SIZE, egui::Sense::hover());
                        ui.interact(rect, trigger, egui::Sense::click());
                    }
                    was_scrolled = ui.ctx().widget_ancestor_scrolled(trigger);
                },
            );
            output.textures_delta.clear();
            assert_eq!(actual_offset, offset);
            was_scrolled
        };
        assert!(!render(ViewportId::ROOT, 0.0, 0.0, "inside"));
        assert!(!render(auxiliary, 0.0, 0.0, "inside"));
        assert!(render(ViewportId::ROOT, OPENED, SCROLLED, "inside"));
        assert!(!render(auxiliary, OPENED, 0.0, "inside"));
        assert!(!render(ViewportId::ROOT, INPUT_TIME, SCROLLED, "inside"));
        assert!(!render(
            ViewportId::ROOT,
            INPUT_TIME,
            SCROLLED + SCROLLED,
            "outside"
        ));
        assert!(!render(
            ViewportId::ROOT,
            INPUT_TIME,
            SCROLLED + SCROLLED,
            "removed"
        ));
        assert!(!render(
            ViewportId::ROOT,
            INPUT_TIME,
            SCROLLED + SCROLLED,
            "inside"
        ));
    }
}

#[test]
fn tooltip_scroll은_직접_저장된_offset과_실제_wheel_변화를_감지한다() {
    for source in ["stored", "wheel"] {
        let context = Context::default();
        let provider = Provider::default();
        let id = Id::new("mutable-scroll-trigger");
        let appearance = Appearance {
            background: Color32::BLACK,
            border: Color32::GRAY,
            foreground: Color32::WHITE,
        };
        let render = |time, events| {
            let mut result = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    ..Default::default()
                },
                |ui| {
                    provider.begin_frame(ui.ctx());
                    let scroll = egui::ScrollArea::vertical()
                        .id_salt("mutable-scroll")
                        .max_width(AREA_WIDTH)
                        .max_height(INNER_HEIGHT)
                        .show(ui, |ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(TRIGGER_SIZE, egui::Sense::hover());
                            let response = ui.interact(rect, id, egui::Sense::click());
                            provider.show(
                                &response,
                                "Mutable scroll",
                                egui::RectAlign::TOP,
                                &appearance,
                            );
                            ui.allocate_space(egui::vec2(AREA_WIDTH, CONTENT_HEIGHT));
                        });
                    result = Some((scroll.id, scroll.state.offset.y, scroll.inner_rect));
                    provider.finish_frame(ui.ctx());
                },
            );
            output.textures_delta.clear();
            result.unwrap()
        };
        render(0.0, Vec::new());
        context.memory_mut(|memory| memory.request_focus(id));
        let (scroll_id, offset, bounds) = render(OPENED, Vec::new());
        assert_eq!(offset, 0.0);
        let events = if source == "stored" {
            let mut state = egui::scroll_area::State::load(&context, scroll_id).unwrap();
            state.offset.y = SCROLLED;
            state.store(&context, scroll_id);
            Vec::new()
        } else {
            vec![
                Event::PointerMoved(bounds.center()),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::Vec2::ZERO,
                    phase: egui::TouchPhase::Start,
                    modifiers: egui::Modifiers::NONE,
                },
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -SCROLLED),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let (_, offset, _) = render(INPUT_TIME, events);
        assert!(offset > 0.0, "fixture {source}");
        let owners = provider.0.lock().unwrap();
        let viewport = owners.get(&ViewportId::ROOT).unwrap();
        assert_eq!(viewport.open, None, "source={source} offset={offset}");
        assert_eq!(viewport.skip_until, Some(INPUT_TIME + SKIP_DELAY_SECONDS));
    }
}
