use super::*;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const TRIGGER: Rect = Rect::from_min_max(egui::pos2(80.0, 100.0), egui::pos2(120.0, 120.0));
const HOVERED: f64 = 0.1;
const OPENED: f64 = 0.5;
const MODAL_MOUNTED: f64 = 0.55;
const MODAL_READY: f64 = 0.6;
const ESCAPED: f64 = 0.7;

#[test]
fn tooltip_escape는_나중에_열린_modal보다_먼저_소비하지_않는다() {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-05-tooltip-escape-source.json"
    ))
    .unwrap();
    let modal = source["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["mode"] == "modal")
        .unwrap();
    assert_eq!(modal["after"]["dialogState"], "closed");
    assert_ne!(modal["after"]["normalState"], "closed");
    let context = Context::default();
    context.enable_accesskit();
    let provider = Provider::default();
    let id = Id::new("under-modal-tooltip");
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let render = |time, has_modal, events| {
        let mut should_close = false;
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                let response = ui.interact(TRIGGER, id, egui::Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Under modal")
                });
                provider.show(&response, "Under modal", egui::RectAlign::TOP, &appearance);
                if has_modal {
                    let modal = egui::Modal::new(Id::new("later-source-modal"))
                        .show(ui.ctx(), |ui| ui.button("Commit"));
                    should_close = modal.should_close();
                }
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        (output, should_close)
    };
    render(0.0, false, Vec::new());
    render(HOVERED, false, vec![Event::PointerMoved(TRIGGER.center())]);
    render(OPENED, false, Vec::new());
    render(MODAL_MOUNTED, true, Vec::new());
    render(MODAL_READY, true, Vec::new());
    let (output, should_close) = render(
        ESCAPED,
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
        should_close,
        "a later modal must receive Escape before an earlier Tooltip"
    );
    let owners = provider.0.lock().unwrap();
    assert_eq!(owners.get(&ViewportId::ROOT).unwrap().open, Some(id));
    let node = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(node_id, _)| *node_id == id.accesskit_id())
        .unwrap();
    assert!(!node.1.described_by().is_empty());
}

#[test]
fn dismissal_layer는_그리기순서와_무관한_마운트순서와_viewport_pass_회수를_보존한다() {
    let context = Context::default();
    context.set_embed_viewports(false);
    let auxiliary = ViewportId(Id::new("dismissal-auxiliary"));
    let tooltip = Id::new("shared-dismissal-tooltip");
    let modal = Id::new("shared-dismissal-modal");
    let render = |viewport, layers: &[Id], removed: Option<Id>, has_auxiliary| {
        let mut current = Vec::new();
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
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                if viewport == ViewportId::ROOT && has_auxiliary {
                    ui.ctx().show_viewport_deferred(
                        auxiliary,
                        egui::ViewportBuilder::default(),
                        |_, _| {},
                    );
                }
                for id in layers {
                    ui.ctx().register_dismissal_layer(*id);
                    ui.ctx().register_dismissal_layer(*id);
                }
                if let Some(id) = removed {
                    ui.ctx().unregister_dismissal_layer(id);
                }
                current = ui.ctx().dismissal_layers();
            },
        );
        output.textures_delta.clear();
        current
    };
    assert_eq!(
        render(ViewportId::ROOT, &[tooltip, modal], None, true),
        [tooltip, modal]
    );
    assert_eq!(
        render(auxiliary, &[modal, tooltip], None, true),
        [modal, tooltip]
    );
    assert_eq!(
        render(ViewportId::ROOT, &[modal, tooltip], None, true),
        [tooltip, modal]
    );
    let during = render(ViewportId::ROOT, &[tooltip], None, true);
    assert_eq!(during, [tooltip, modal]);
    assert_eq!(context.dismissal_layers(), [tooltip]);
    assert_eq!(
        render(ViewportId::ROOT, &[tooltip, modal], Some(modal), true),
        [tooltip]
    );
    assert_eq!(
        render(ViewportId::ROOT, &[modal, tooltip], None, true),
        [tooltip, modal]
    );
    assert_eq!(
        render(auxiliary, &[tooltip, modal], None, true),
        [modal, tooltip]
    );
    render(ViewportId::ROOT, &[], None, false);
    assert!(context.dismissal_layers().is_empty());
    assert!(render(auxiliary, &[], None, true).is_empty());
    assert!(Context::default().dismissal_layers().is_empty());
}
