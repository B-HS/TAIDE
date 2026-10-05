use super::*;
use std::path::Path;
use taide_lsp::native::protocol::lsp_types::{DiagnosticSeverity, Position, Range};
use taide_model::ids::TabId;
use taide_native_editor::{
    document::DocumentKey,
    store::{EditorLimits, EditorStore},
};

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 220.0;
const DIAGNOSTIC_COUNT: usize = 1000;
const BYTE_LIMIT: usize = 1024;
const SCROLL_DISTANCE: f32 = 1000.0;

fn pointer(position: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(WIDTH, HEIGHT))),
        events,
        ..Default::default()
    }
}

fn texts(output: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
    fn collect(shape: &egui::Shape, result: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::Shape::Text(text) => result.push((
                text.galley.text().into(),
                text.pos + text.galley.size() / 2.0,
            )),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, result);
                }
            }
            _ => {}
        }
    }
    let mut result = Vec::new();
    for shape in &output.shapes {
        collect(&shape.shape, &mut result);
    }
    result
}

fn panel_frame(
    context: &egui::Context,
    views: &mut Views,
    slot: &ShellSlotId,
    store: &Store,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Vec<Open>, Option<Id>) {
    let mut scroll_id = None;
    let mut open = Vec::new();
    let mut output = context.run_ui(input(events), |ui| {
        open = views
            .show_panel(ui, slot, store, locale, appearance)
            .unwrap();
        scroll_id = views.panels.get(slot).and_then(|panel| {
            panel.viewport.as_ref().map(|viewport| {
                ui.make_persistent_id(egui::IdSalt::new(("problems", slot, Some(viewport.mount))))
            })
        });
    });
    output.textures_delta.clear();
    (output, open, scroll_id)
}

fn batch_appearance_locale() -> (Appearance, ResolvedLocale) {
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-problems-batch-{}", uuid::Uuid::new_v4())),
    ));
    let appearance = Appearance::new(
        &taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap(),
    )
    .unwrap();
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    (appearance, locale)
}

fn batch_keys(keys: &[(egui::Key, bool, bool)]) -> Vec<egui::Event> {
    keys.iter()
        .map(|&(key, pressed, repeat)| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat,
            modifiers: egui::Modifiers::NONE,
        })
        .collect()
}

fn batch_store(path: &Path, count: usize) -> Store {
    batch_store_diagnostics(path, vec![Diagnostic::default(); count])
}

fn batch_store_diagnostics(path: &Path, diagnostics: Vec<Diagnostic>) -> Store {
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_document_bytes: BYTE_LIMIT,
        max_undo_groups: 1,
    })
    .unwrap();
    let document = editor
        .open_untitled(TabId::new(), "batch", "rust".into())
        .unwrap();
    let mut snapshot = editor.documents().snapshot(document).unwrap();
    snapshot.key = DocumentKey::File(path.to_path_buf());
    let owner = diagnostics::Owner::new();
    let mut store = Store::default();
    store.retain_documents(HashSet::from([document]));
    store.reconcile(HashMap::from([(owner, HashSet::from([document]))]));
    store.publish(owner, &snapshot, diagnostics);
    store
}

#[test]
fn problems_비활성_filter의_hover_배경에도_원본_opacity를_적용한다() {
    const SOURCE_INACTIVE_OPACITY: f32 = 0.6;
    let (appearance, locale) = batch_appearance_locale();
    let store = batch_store(Path::new("/synthetic/opacity.rs"), 1);
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let (output, _, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        Vec::new(),
    );
    let label = crate::presentation::message(&locale, "problems.severity.error", &[]);
    let (id, node) = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label.as_str()))
        .unwrap();
    let bounds = node.bounds().unwrap();
    let rect = Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    );
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: *id,
                data: None,
            },
        )],
    );
    assert!(!views.panels[&slot].active[diagnostics::ERROR]);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        vec![egui::Event::PointerMoved(rect.center())],
    );
    let (hovered, _, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        Vec::new(),
    );
    let background = hovered
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(shape) if shape.rect == rect => Some(shape),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        background.fill,
        appearance.hover.gamma_multiply(SOURCE_INACTIVE_OPACITY)
    );
    let text = hovered
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "1" && rect.contains(text.pos) => {
                Some(text)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        text.fallback_color,
        appearance.muted.gamma_multiply(SOURCE_INACTIVE_OPACITY)
    );
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: *id,
                data: None,
            },
        )],
    );
    let (selected, _, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        Vec::new(),
    );
    assert!(selected.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Rect(shape) if shape.rect == rect && shape.fill == appearance.selected)
    }));
}

#[test]
fn problems_tooltip은_키보드_focus에_즉시_열리고_escape와_blur에_닫힌다() {
    const FOCUSED: f64 = 1.0;
    const CLOSED: f64 = 2.0;
    const WARNING_FOCUSED: f64 = 3.0;
    const MIXED: f64 = 4.0;
    const DURATION: f64 = 0.15;
    let (appearance, locale) = batch_appearance_locale();
    let store = Store::default();
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let frame = |views: &mut Views, time, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                ..input(events)
            },
            |ui| {
                views.tooltips.begin_frame(ui.ctx());
                ui.add_space(HEIGHT / 2.0);
                views
                    .show_panel(ui, &slot, &store, &locale, &appearance)
                    .unwrap();
                views.tooltips.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        output
    };
    let initial = frame(&mut views, 0.0, Vec::new());
    let error = crate::presentation::message(&locale, "problems.severity.error", &[]);
    let warning = crate::presentation::message(&locale, "problems.severity.warning", &[]);
    let nodes = &initial
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let node = |label: &str| {
        nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label))
            .unwrap()
            .0
    };
    let focus = |target_node| {
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Focus,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node,
            data: None,
        })
    };
    let focused = frame(&mut views, FOCUSED, vec![focus(node(&error))]);
    assert_eq!(
        focused
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .focus,
        node(&error)
    );
    assert!(
        focused
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
    );
    let focused = frame(&mut views, FOCUSED + DURATION, Vec::new());
    assert!(texts(&focused).iter().any(|(text, _)| text == &error));
    let escaped = frame(
        &mut views,
        CLOSED,
        batch_keys(&[(egui::Key::Escape, true, false)]),
    );
    assert!(
        escaped
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
    );
    let still_focused = frame(&mut views, CLOSED + DURATION, Vec::new());
    assert!(!texts(&still_focused).iter().any(|(text, _)| text == &error));
    let blurred = frame(&mut views, WARNING_FOCUSED, vec![focus(node(&warning))]);
    assert!(!texts(&blurred).iter().any(|(text, _)| text == &error));
    let blurred = frame(&mut views, WARNING_FOCUSED + DURATION, Vec::new());
    assert!(texts(&blurred).iter().any(|(text, _)| text == &warning));
    let mut mixed = vec![focus(node(&error))];
    mixed.extend(batch_keys(&[(egui::Key::Enter, true, false)]));
    mixed.push(focus(node(&warning)));
    frame(&mut views, MIXED, mixed);
    let latest = frame(&mut views, MIXED + DURATION, Vec::new());
    assert!(!texts(&latest).iter().any(|(text, _)| text == &error));
    assert!(!texts(&latest).iter().any(|(text, _)| text == &warning));
}

#[test]
fn problems와_ide_status는_실제_caller에서_tooltip_skip을_공유한다() {
    const DELAY: f64 = 0.4;
    const CLOSED: f64 = 0.6;
    const QUICK_MOVE: f64 = 0.7;
    const DURATION: f64 = 0.15;
    const PORT: u32 = 12345;
    let (appearance, locale) = batch_appearance_locale();
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-tooltip-shared-{}", uuid::Uuid::new_v4())),
    ));
    let theme =
        taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
    let ide_appearance = crate::status_ide::Appearance::new(&theme).unwrap();
    let context = egui::Context::default();
    context.enable_accesskit();
    let provider = crate::tooltips::Provider::default();
    let mut views = Views::with_tooltips("en-US", provider.clone()).unwrap();
    let mut icons = crate::status_ide::Icons::with_tooltips(provider.clone()).unwrap();
    let store = Store::default();
    let slot = ShellSlotId::new();
    let mut bounds = [Rect::NOTHING; 2];
    let mut render = |time, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                ..input(events)
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                ui.add_space(HEIGHT / 2.0);
                ui.horizontal(|ui| {
                    bounds[0] = views
                        .show_status(ui, Some(&slot), &store, &locale, &appearance)
                        .unwrap()
                        .rect;
                    bounds[1] = crate::status_ide::show(
                        ui,
                        &locale,
                        &ide_appearance,
                        &mut icons,
                        taide_model::ide::IdeStatus {
                            running: true,
                            connected: true,
                            port: PORT,
                            client_count: 1,
                        },
                    )
                    .unwrap()
                    .rect;
                });
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        (output, bounds)
    };
    let (_, initial_bounds) = render(0.0, Vec::new());
    let label = crate::presentation::message(&locale, "problems.toggleAriaLabel", &[]);
    render(
        0.0,
        vec![egui::Event::PointerMoved(initial_bounds[0].center())],
    );
    render(DELAY, Vec::new());
    let (opening, _) = render(DELAY, Vec::new());
    assert!(
        opening
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip
                && node.label() == Some(label.as_str()))
    );
    let (opened, _) = render(DELAY + DURATION, Vec::new());
    assert!(texts(&opened).iter().any(|(text, _)| text == &label));
    let (closed, bounds) = render(CLOSED, batch_keys(&[(egui::Key::Escape, true, false)]));
    assert!(
        closed
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip
                && node.label() == Some(label.as_str()))
    );
    render(
        QUICK_MOVE,
        vec![egui::Event::PointerMoved(bounds[1].center())],
    );
    let title = crate::presentation::message(&locale, "ide.title", &[("port", &PORT.to_string())]);
    let (opening, _) = render(QUICK_MOVE, Vec::new());
    assert!(
        opening
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip
                && node.label() == Some(title.as_str()))
    );
    let (ide, _) = render(QUICK_MOVE + DURATION, Vec::new());
    assert!(texts(&ide).iter().any(|(text, _)| text == &title));
    assert!(!texts(&ide).iter().any(|(text, _)| text == &label));
}

#[test]
fn problems_tooltip은_ax_role과_trigger_description을_열린_동안만_연결한다() {
    const OPENED: f64 = 1.0;
    const CLOSED: f64 = 2.0;
    const DURATION: f64 = 0.15;
    let (appearance, locale) = batch_appearance_locale();
    let context = egui::Context::default();
    context.enable_accesskit();
    let store = Store::default();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    let label = crate::presentation::message(&locale, "problems.toggleAriaLabel", &[]);
    let frame = |views: &mut Views, time, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                ..input(events)
            },
            |ui| {
                ui.add_space(HEIGHT / 2.0);
                views
                    .show_status(ui, Some(&slot), &store, &locale, &appearance)
                    .unwrap();
            },
        );
        output.textures_delta.clear();
        output
    };
    let initial = frame(&mut views, 0.0, Vec::new());
    let target = initial
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == egui::accesskit::Role::Button && node.label() == Some(label.as_str())
        })
        .unwrap()
        .0;
    frame(
        &mut views,
        OPENED,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_node: target,
                target_tree: egui::accesskit::TreeId::ROOT,
                action: egui::accesskit::Action::Focus,
                data: None,
            },
        )],
    );
    let opened = frame(&mut views, OPENED + DURATION, Vec::new());
    let nodes = &opened
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let tooltips = nodes
        .iter()
        .filter(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
        .collect::<Vec<_>>();
    assert_eq!(tooltips.len(), 1);
    let (tooltip_id, tooltip) = tooltips[0];
    assert_eq!(tooltip.label(), Some(label.as_str()));
    assert!(tooltip.bounds().unwrap().width() > 0.0);
    assert!(!tooltip.supports_action(egui::accesskit::Action::Focus));
    assert!(
        tooltip.children().iter().any(|child| {
            nodes.iter().any(|(id, node)| {
                id == child
                    && node.role() == egui::accesskit::Role::GenericContainer
                    && node.children().iter().any(|text| {
                        nodes.iter().any(|(id, node)| {
                            id == text
                                && node.role() == egui::accesskit::Role::Label
                                && node.value() == Some(label.as_str())
                        })
                    })
            })
        }),
        "{nodes:?}"
    );
    let trigger = nodes.iter().find(|(id, _)| *id == target).unwrap();
    assert_eq!(trigger.1.role(), egui::accesskit::Role::Button);
    assert_eq!(trigger.1.described_by(), &[*tooltip_id]);
    assert!(texts(&opened).iter().any(|(text, _)| text == &label));
    let closed = frame(
        &mut views,
        CLOSED,
        batch_keys(&[(egui::Key::Escape, true, false)]),
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
            .find(|(id, _)| *id == target)
            .unwrap()
            .1
            .described_by()
            .is_empty()
    );
    let removed = frame(&mut views, CLOSED + DURATION, Vec::new());
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
fn problems_tooltip은_테마색과_원본_글꼴_여백_테두리를_렌더한다() {
    const SOURCE_FONT_SIZE: f32 = 12.0;
    const SOURCE_LINE_HEIGHT: f32 = 16.0;
    const SOURCE_RADIUS: u8 = 6;
    const SOURCE_HORIZONTAL_MARGIN: f32 = 12.0;
    const SOURCE_VERTICAL_MARGIN: f32 = 6.0;
    const PIXEL_ROUNDING: f32 = 1.0 / 32.0;
    let (_, locale) = batch_appearance_locale();
    for theme in ["vscode-dark-modern", "vscode-light-modern"] {
        let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
            std::env::temp_dir().join(format!("taide-tooltip-theme-{}", uuid::Uuid::new_v4())),
        ));
        let theme = taide_runtime::theme_actions::theme_get(&state, theme.into()).unwrap();
        let background_color = crate::presentation::color(&theme, "tooltip.background").unwrap();
        let border_color = crate::presentation::color(&theme, "tooltip.border").unwrap();
        let appearance = Appearance::new(&theme).unwrap();
        let store = Store::default();
        let context = egui::Context::default();
        context.enable_accesskit();
        let slot = ShellSlotId::new();
        let mut views = Views::new("en-US").unwrap();
        let label = crate::presentation::message(&locale, "problems.toggleAriaLabel", &[]);
        let frame = |views: &mut Views, time, events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    ..input(events)
                },
                |ui| {
                    ui.add_space(HEIGHT / 2.0);
                    views
                        .show_status(ui, Some(&slot), &store, &locale, &appearance)
                        .unwrap();
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = frame(&mut views, 0.0, Vec::new());
        let bounds = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|(_, node)| {
                (node.label() == Some(label.as_str())).then(|| node.bounds().unwrap())
            })
            .unwrap();
        let center = egui::pos2(
            (bounds.x0 + bounds.x1) as f32 / 2.0,
            (bounds.y0 + bounds.y1) as f32 / 2.0,
        );
        frame(&mut views, 1.0, vec![egui::Event::PointerMoved(center)]);
        let ready = 1.0 + f64::from(context.global_style().interaction.tooltip_delay) + 1.0;
        frame(&mut views, ready, Vec::new());
        let hovered = frame(&mut views, ready + 1.0, Vec::new());
        let background = hovered
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(shape) if shape.fill == background_color => Some(shape),
                _ => None,
            })
            .expect("tooltip must use the resolved tooltip.background");
        assert_eq!(background.stroke, Stroke::new(1.0, border_color));
        assert_eq!(
            background.corner_radius,
            egui::CornerRadius::same(SOURCE_RADIUS)
        );
        let text = hovered
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => Some(text),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            text.galley.job.sections[0].format.color,
            appearance.foreground
        );
        assert_eq!(
            text.galley.job.sections[0].format.font_id,
            FontId::proportional(SOURCE_FONT_SIZE)
        );
        assert_eq!(
            text.galley.job.sections[0].format.line_height,
            Some(SOURCE_LINE_HEIGHT)
        );
        let expected = text.galley.size()
            + vec2(
                SOURCE_HORIZONTAL_MARGIN * 2.0 + 2.0,
                SOURCE_VERTICAL_MARGIN * 2.0 + 2.0,
            );
        assert!((background.rect.size() - expected).abs().max_elem() <= PIXEL_ROUNDING);
    }
}

#[test]
fn problems_버튼은_원본_모서리와_높이_툴팁방향을_렌더한다() {
    const SOURCE_RADIUS: u8 = 4;
    const SOURCE_STATUS_HEIGHT: f32 = 16.5;
    const HOVER_START: f64 = 1.0;
    const SETTLE_SECONDS: f64 = 1.0;
    let (appearance, locale) = batch_appearance_locale();
    let store = Store::default();
    for key in [
        "problems.toggleAriaLabel",
        "problems.severity.error",
        "common.close",
    ] {
        let status = key == "problems.toggleAriaLabel";
        let context = egui::Context::default();
        context.enable_accesskit();
        let slot = ShellSlotId::new();
        let mut views = Views::new("en-US").unwrap();
        views.toggle(&slot);
        let label = crate::presentation::message(&locale, key, &[]);
        let frame = |views: &mut Views, time, events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    ..input(events)
                },
                |ui| {
                    ui.add_space(HEIGHT / 2.0);
                    if status {
                        views
                            .show_status(ui, Some(&slot), &store, &locale, &appearance)
                            .unwrap();
                    } else {
                        views
                            .show_panel(ui, &slot, &store, &locale, &appearance)
                            .unwrap();
                    }
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = frame(&mut views, 0.0, Vec::new());
        let bounds = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|(_, node)| {
                (node.label() == Some(label.as_str())).then(|| node.bounds().unwrap())
            })
            .unwrap();
        let rect = Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        );
        frame(
            &mut views,
            HOVER_START,
            vec![egui::Event::PointerMoved(rect.center())],
        );
        let ready = HOVER_START
            + f64::from(context.global_style().interaction.tooltip_delay)
            + SETTLE_SECONDS;
        frame(&mut views, ready, Vec::new());
        let hovered = frame(&mut views, ready + SETTLE_SECONDS, Vec::new());
        let fill = if key == "common.close" {
            appearance.hover
        } else {
            appearance.selected
        };
        let background = hovered
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(shape) if shape.rect == rect && shape.fill == fill => Some(shape),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            background.corner_radius,
            egui::CornerRadius::same(SOURCE_RADIUS),
            "{key}"
        );
        assert_eq!(
            rect.height(),
            if status {
                SOURCE_STATUS_HEIGHT
            } else {
                ROW_HEIGHT
            },
            "{key}"
        );
        let tooltip = texts(&hovered)
            .into_iter()
            .find(|(text, _)| text == &label)
            .unwrap();
        assert!(
            if status {
                tooltip.1.y < rect.top()
            } else {
                tooltip.1.y > rect.bottom()
            },
            "{key}: {tooltip:?}, {rect:?}"
        );
    }
}

#[test]
fn problems_서로다른_진단행의_ax_click은_원래_사건순서를_보존한다() {
    let (appearance, locale) = batch_appearance_locale();
    let path = PathBuf::from("/synthetic/ordered.rs");
    let store = batch_store_diagnostics(
        &path,
        [0, 1]
            .into_iter()
            .map(|line| Diagnostic {
                range: Range::new(Position::new(line, 0), Position::new(line, 1)),
                ..Default::default()
            })
            .collect(),
    );
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let frame = |views: &mut Views, events| {
        panel_frame(&context, views, &slot, &store, &locale, &appearance, events)
    };
    let (output, _, _) = frame(&mut views, Vec::new());
    let rows = [0usize, 1].map(|index| Id::new(("problem-row", &slot, &path, Some(index))));
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    assert!(
        rows.iter()
            .all(|row| nodes.iter().any(|(id, _)| *id == row.accesskit_id()))
    );
    let events = [rows[1], rows[0], rows[1]]
        .into_iter()
        .map(|row| {
            egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: row.accesskit_id(),
                data: None,
            })
        })
        .collect();
    let (_, opened, _) = frame(&mut views, events);
    assert_eq!(
        opened.iter().map(|open| open.line).collect::<Vec<_>>(),
        [2, 1, 2]
    );
    assert!(
        opened
            .iter()
            .all(|open| open.path == "/synthetic/ordered.rs" && open.column == 1)
    );
}

#[test]
fn problems_진단행의_pointer_key_ax_혼합은_원래_사건순서를_보존한다() {
    let (appearance, locale) = batch_appearance_locale();
    let path = PathBuf::from("/synthetic/mixed.rs");
    let store = batch_store_diagnostics(
        &path,
        [0, 1]
            .into_iter()
            .map(|line| Diagnostic {
                message: format!("mixed row {line}"),
                range: Range::new(Position::new(line, 0), Position::new(line, 1)),
                ..Default::default()
            })
            .collect(),
    );
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let frame = |views: &mut Views, events| {
        panel_frame(&context, views, &slot, &store, &locale, &appearance, events)
    };
    let (output, _, _) = frame(&mut views, Vec::new());
    let position = texts(&output)
        .into_iter()
        .find(|(text, _)| text == "mixed row 1")
        .unwrap()
        .1;
    let rows = [0usize, 1].map(|index| Id::new(("problem-row", &slot, &path, Some(index))));
    let click = |row: Id| {
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: row.accesskit_id(),
            data: None,
        })
    };
    frame(&mut views, Vec::new());
    assert!(frame(&mut views, pointer(position, true)).1.is_empty());
    let events = std::iter::once(click(rows[0]))
        .chain(pointer(position, false))
        .chain(std::iter::once(click(rows[0])))
        .collect();
    let (_, opened, _) = frame(&mut views, events);
    assert_eq!(
        opened.iter().map(|open| open.line).collect::<Vec<_>>(),
        [1, 2, 1]
    );
    context.memory_mut(|memory| memory.request_focus(rows[1]));
    frame(&mut views, Vec::new());
    let events = batch_keys(&[(egui::Key::Enter, true, false)])
        .into_iter()
        .chain(std::iter::once(click(rows[0])))
        .chain(batch_keys(&[(egui::Key::Enter, true, true)]))
        .collect();
    let (_, opened, _) = frame(&mut views, events);
    assert_eq!(
        opened.iter().map(|open| open.line).collect::<Vec<_>>(),
        [2, 1, 2]
    );
    assert!(
        opened
            .iter()
            .all(|open| open.path == "/synthetic/mixed.rs" && open.column == 1)
    );
    assert!(!context.input(|input| {
        input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Key {
                    key: egui::Key::Enter,
                    ..
                } | egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    ..
                })
            )
        })
    }));
}

#[test]
fn problems_같은프레임_ax_focus_전환은_enter를_사건별_진단행에_전달한다() {
    let (appearance, locale) = batch_appearance_locale();
    let path = PathBuf::from("/synthetic/focus.rs");
    let store = batch_store_diagnostics(
        &path,
        [0, 1]
            .into_iter()
            .map(|line| Diagnostic {
                range: Range::new(Position::new(line, 0), Position::new(line, 1)),
                ..Default::default()
            })
            .collect(),
    );
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let frame = |views: &mut Views, events| {
        panel_frame(&context, views, &slot, &store, &locale, &appearance, events)
    };
    let (output, _, _) = frame(&mut views, Vec::new());
    let rows = [0usize, 1].map(|index| Id::new(("problem-row", &slot, &path, Some(index))));
    assert!(rows.iter().all(|row| {
        output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(id, _)| *id == row.accesskit_id())
    }));
    context.memory_mut(|memory| memory.request_focus(rows[0]));
    frame(&mut views, Vec::new());
    let events = [rows[1], rows[0]]
        .into_iter()
        .flat_map(|row| {
            std::iter::once(egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: row.accesskit_id(),
                    data: None,
                },
            ))
            .chain(batch_keys(&[
                (egui::Key::Enter, true, false),
                (egui::Key::Enter, false, false),
            ]))
        })
        .collect();
    let (_, opened, _) = frame(&mut views, events);
    assert_eq!(
        opened.iter().map(|open| open.line).collect::<Vec<_>>(),
        [2, 1]
    );
    assert!(context.memory(|memory| memory.has_focus(rows[0])));
    assert!(!context.input(|input| input.key_pressed(egui::Key::Enter)));
}

#[test]
fn problems_filter_collapse_close는_진단열기와_원래_사건순서로_적용된다() {
    let (appearance, locale) = batch_appearance_locale();
    let path = PathBuf::from("/synthetic/actions.rs");
    let store = batch_store(&path, 1);
    for (action, control_first, should_open) in [
        ("filter", false, true),
        ("filter", true, false),
        ("collapse", false, true),
        ("collapse", true, false),
        ("close", false, true),
        ("close", true, false),
    ] {
        let context = egui::Context::default();
        context.enable_accesskit();
        let slot = ShellSlotId::new();
        let mut views = Views::new("en-US").unwrap();
        views.toggle(&slot);
        let (output, _, _) = panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            Vec::new(),
        );
        let row = Id::new(("problem-row", &slot, &path, Some(0usize))).accesskit_id();
        let group = Id::new(("problem-row", &slot, &path, None::<usize>)).accesskit_id();
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        assert!(nodes.iter().any(|(id, _)| *id == row));
        let label = match action {
            "filter" => crate::presentation::message(&locale, "problems.severity.error", &[]),
            "close" => crate::presentation::message(&locale, "common.close", &[]),
            "collapse" => path.to_string_lossy().into_owned(),
            _ => unreachable!(),
        };
        let target = if action == "collapse" {
            assert!(nodes.iter().any(|(id, _)| *id == group));
            group
        } else {
            nodes
                .iter()
                .find(|(_, node)| node.label() == Some(label.as_str()))
                .unwrap()
                .0
        };
        let order = if control_first {
            [target, row]
        } else {
            [row, target]
        };
        let events = order
            .into_iter()
            .map(|target_node| {
                egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node,
                    data: None,
                })
            })
            .collect();
        let (_, opened, _) = panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            events,
        );
        assert_eq!(
            opened.len(),
            usize::from(should_open),
            "{action}, control_first={control_first}"
        );
        assert!(opened.iter().all(|open| {
            open.path == "/synthetic/actions.rs" && open.line == 1 && open.column == 1
        }));
        match action {
            "filter" => {
                assert!(!views.panels[&slot].active[diagnostics::ERROR]);
                assert!(views.panels[&slot].rows.is_empty());
            }
            "collapse" => assert_eq!(views.panels[&slot].rows.len(), 1),
            "close" => assert!(!views.is_open(&slot)),
            _ => unreachable!(),
        }
    }
}

#[test]
fn problems_필터변경은_같은_행id의_새_진단을_열고_사라진_행을_거절한다() {
    let (appearance, locale) = batch_appearance_locale();
    let path = PathBuf::from("/synthetic/retarget.rs");
    let store = batch_store_diagnostics(
        &path,
        vec![
            Diagnostic {
                severity: Some(DiagnosticSeverity::ERROR),
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                ..Default::default()
            },
            Diagnostic {
                severity: Some(DiagnosticSeverity::WARNING),
                range: Range::new(Position::new(1, 3), Position::new(1, 4)),
                ..Default::default()
            },
        ],
    );
    for (order, expected) in [
        (vec!["row0", "filter", "row0"], vec![(1, 1), (2, 4)]),
        (vec!["filter", "row1"], Vec::new()),
        (
            vec!["filter", "row0", "filter", "row0"],
            vec![(2, 4), (1, 1)],
        ),
    ] {
        let context = egui::Context::default();
        context.enable_accesskit();
        let slot = ShellSlotId::new();
        let mut views = Views::new("en-US").unwrap();
        views.toggle(&slot);
        let (output, _, _) = panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            Vec::new(),
        );
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let error = crate::presentation::message(&locale, "problems.severity.error", &[]);
        let filter = nodes
            .iter()
            .find(|(_, node)| node.label() == Some(error.as_str()))
            .unwrap()
            .0;
        let rows = [0usize, 1]
            .map(|index| Id::new(("problem-row", &slot, &path, Some(index))).accesskit_id());
        assert!(rows.iter().all(|row| nodes.iter().any(|(id, _)| id == row)));
        let events = order
            .iter()
            .map(|target| {
                let target_node = match *target {
                    "row0" => rows[0],
                    "row1" => rows[1],
                    "filter" => filter,
                    _ => unreachable!(),
                };
                egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node,
                    data: None,
                })
            })
            .collect();
        let (_, opened, _) = panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            events,
        );
        assert_eq!(
            opened
                .iter()
                .map(|open| (open.line, open.column))
                .collect::<Vec<_>>(),
            expected,
            "{order:?}"
        );
        assert!(
            opened
                .iter()
                .all(|open| open.path == "/synthetic/retarget.rs")
        );
    }
}

#[test]
fn problems_viewport별_scroll은_독립적이며_제거와_slot_회수시_폐기된다() {
    let (appearance, locale) = batch_appearance_locale();
    let store = batch_store(Path::new("/synthetic/viewports.rs"), DIAGNOSTIC_COUNT);
    let context = egui::Context::default();
    let root = egui::ViewportId::ROOT;
    let child = egui::ViewportId::from_hash_of("problems-child");
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let run = |views: &mut Views, viewport, live: &[egui::ViewportId], events| {
        let mut scroll_id = None;
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: viewport,
                viewports: live
                    .iter()
                    .map(|id| {
                        (
                            *id,
                            egui::ViewportInfo {
                                parent: (*id != root).then_some(root),
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
                ..input(events)
            },
            |ui| {
                assert!(
                    views
                        .show_panel(ui, &slot, &store, &locale, &appearance)
                        .unwrap()
                        .is_empty()
                );
                scroll_id = views.panels[&slot].viewport.as_ref().map(|owner| {
                    ui.make_persistent_id(egui::IdSalt::new(("problems", &slot, Some(owner.mount))))
                });
            },
        );
        output.textures_delta.clear();
        (output, scroll_id.unwrap())
    };
    let wheel = |phase, delta| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta,
        phase,
        modifiers: egui::Modifiers::NONE,
    };
    let live = [root, child];
    let (_, root_id) = run(&mut views, root, &live, Vec::new());
    let (_, child_id) = run(&mut views, child, &live, Vec::new());
    assert_ne!(root_id, child_id);
    run(
        &mut views,
        root,
        &live,
        vec![
            egui::Event::PointerMoved(egui::pos2(WIDTH / 2.0, HEIGHT / 2.0)),
            wheel(egui::TouchPhase::Start, egui::Vec2::ZERO),
            wheel(egui::TouchPhase::Move, vec2(0.0, -SCROLL_DISTANCE)),
        ],
    );
    run(
        &mut views,
        root,
        &live,
        vec![wheel(egui::TouchPhase::End, egui::Vec2::ZERO)],
    );
    let root_offset = egui::scroll_area::State::load(&context, root_id)
        .unwrap()
        .offset;
    assert!(root_offset.y > 0.0);
    run(&mut views, child, &live, Vec::new());
    assert_eq!(
        egui::scroll_area::State::load(&context, child_id)
            .unwrap()
            .offset
            .y,
        0.0
    );
    run(
        &mut views,
        child,
        &live,
        vec![
            egui::Event::PointerMoved(egui::pos2(WIDTH / 2.0, HEIGHT / 2.0)),
            wheel(egui::TouchPhase::Start, egui::Vec2::ZERO),
            wheel(egui::TouchPhase::Move, vec2(0.0, -SCROLL_DISTANCE / 2.0)),
        ],
    );
    run(
        &mut views,
        child,
        &live,
        vec![wheel(egui::TouchPhase::End, egui::Vec2::ZERO)],
    );
    let child_offset = egui::scroll_area::State::load(&context, child_id)
        .unwrap()
        .offset;
    assert!(child_offset.y > 0.0);
    assert_ne!(root_offset, child_offset);
    assert_eq!(
        egui::scroll_area::State::load(&context, root_id)
            .unwrap()
            .offset,
        root_offset
    );
    run(&mut views, root, &[root], Vec::new());
    assert!(egui::scroll_area::State::load(&context, child_id).is_none());
    assert_eq!(
        views.panels[&slot].viewport.as_ref().unwrap().states.len(),
        1
    );
    assert_eq!(
        egui::scroll_area::State::load(&context, root_id)
            .unwrap()
            .offset,
        root_offset
    );
    let (recreated, new_child_id) = run(&mut views, child, &live, Vec::new());
    assert_eq!(new_child_id, child_id);
    assert_eq!(
        egui::scroll_area::State::load(&context, new_child_id)
            .unwrap()
            .offset
            .y,
        0.0
    );
    assert!(
        texts(&recreated)
            .iter()
            .any(|(text, _)| text == "viewports.rs")
    );
    views.reconcile(None);
    assert!(!views.is_open(&slot));
    for id in [root_id, new_child_id] {
        assert!(egui::scroll_area::State::load(&context, id).is_none());
    }
}

#[test]
fn problems_프레임_batch_행은_최초_space와_enter_사건수를_보존한다() {
    let (appearance, locale) = batch_appearance_locale();
    let path = PathBuf::from("/synthetic/batch.rs");
    let store = batch_store(&path, 1);
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let run = |views: &mut Views, keys: &[(egui::Key, bool, bool)]| {
        panel_frame(
            &context,
            views,
            &slot,
            &store,
            &locale,
            &appearance,
            batch_keys(keys),
        )
    };
    run(&mut views, &[]);
    let group = Id::new(("problem-row", &slot, &path, None::<usize>));
    context.memory_mut(|memory| memory.request_focus(group));
    run(&mut views, &[]);
    run(
        &mut views,
        &[
            (egui::Key::Space, true, false),
            (egui::Key::Space, true, true),
        ],
    );
    assert_eq!(views.panels[&slot].rows.len(), 1);
    run(&mut views, &[(egui::Key::Space, false, false)]);
    run(
        &mut views,
        &[
            (egui::Key::Enter, true, false),
            (egui::Key::Enter, true, true),
        ],
    );
    assert_eq!(views.panels[&slot].rows.len(), 1);
    run(&mut views, &[(egui::Key::Enter, false, false)]);
    run(
        &mut views,
        &[
            (egui::Key::Enter, true, false),
            (egui::Key::Enter, true, true),
            (egui::Key::Enter, true, true),
        ],
    );
    assert_eq!(
        views.panels[&slot].rows.len(),
        store.counts()[diagnostics::ERROR] + 1
    );
    run(&mut views, &[(egui::Key::Enter, false, false)]);
    run(
        &mut views,
        &[
            (egui::Key::Space, true, false),
            (egui::Key::Space, false, false),
            (egui::Key::Space, true, false),
            (egui::Key::Space, false, false),
        ],
    );
    assert_eq!(
        views.panels[&slot].rows.len(),
        store.counts()[diagnostics::ERROR] + 1
    );
    assert!(!context.input(|input| input.key_pressed(egui::Key::Space)));
    let row = Id::new(("problem-row", &slot, &path, Some(0usize)));
    context.memory_mut(|memory| memory.request_focus(row));
    let (output, _, _) = run(&mut views, &[]);
    assert!(
        output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(id, _)| *id == row.accesskit_id())
    );
    let keys = [
        (egui::Key::Enter, true, false),
        (egui::Key::Enter, true, true),
    ];
    let (_, opens, _) = run(&mut views, &keys);
    assert_eq!(opens.len(), keys.len());
    assert!(
        opens
            .iter()
            .all(|open| (open.path.as_str(), open.line, open.column)
                == ("/synthetic/batch.rs", 1, 1))
    );
    run(&mut views, &[(egui::Key::Enter, false, false)]);
    let requests = vec![
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: row.accesskit_id(),
            data: None,
        }),
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: row.accesskit_id(),
            data: None,
        }),
    ];
    let expected = requests.len();
    let (_, opens, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        requests,
    );
    assert_eq!(opens.len(), expected);
}

#[test]
fn problems_프레임_batch_헤더는_짝수_enter와_space_해제의_토글수를_보존한다() {
    let (appearance, locale) = batch_appearance_locale();
    let context = egui::Context::default();
    let slot = ShellSlotId::new();
    let store = Store::default();
    let mut views = Views::new("en-US").unwrap();
    let run = |views: &mut Views, keys: &[(egui::Key, bool, bool)]| {
        let mut response = None;
        let mut output = context.run_ui(input(batch_keys(keys)), |ui| {
            response = Some(
                views
                    .show_status(ui, Some(&slot), &store, &locale, &appearance)
                    .unwrap(),
            );
        });
        output.textures_delta.clear();
        response.unwrap()
    };
    let response = run(&mut views, &[]);
    context.memory_mut(|memory| memory.request_focus(response.id));
    run(&mut views, &[]);
    run(
        &mut views,
        &[
            (egui::Key::Enter, true, false),
            (egui::Key::Enter, true, true),
        ],
    );
    assert!(!views.is_open(&slot));
    run(&mut views, &[(egui::Key::Enter, false, false)]);
    run(
        &mut views,
        &[
            (egui::Key::Space, true, false),
            (egui::Key::Space, true, true),
            (egui::Key::Space, false, false),
            (egui::Key::Space, true, false),
            (egui::Key::Space, false, false),
        ],
    );
    assert!(!views.is_open(&slot));
    run(
        &mut views,
        &[
            (egui::Key::Enter, true, false),
            (egui::Key::Enter, true, true),
            (egui::Key::Enter, true, true),
        ],
    );
    assert!(views.is_open(&slot));
    run(&mut views, &[(egui::Key::Enter, false, false)]);
    views.panels.get_mut(&slot).unwrap().active[diagnostics::ERROR] = false;
    run(
        &mut views,
        &[
            (egui::Key::Enter, true, false),
            (egui::Key::Enter, true, true),
        ],
    );
    assert!(views.is_open(&slot));
    assert!(views.panels[&slot].active[diagnostics::ERROR]);
    run(&mut views, &[(egui::Key::Enter, false, false)]);
    run(
        &mut views,
        &[
            (egui::Key::Space, true, false),
            (egui::Key::Space, false, false),
        ],
    );
    assert!(!views.is_open(&slot));
}

#[test]
fn problems_빈필터_왕복은_실제_scroll과_viewport_수명을_초기화한다() {
    let (appearance, locale) = batch_appearance_locale();
    let path = PathBuf::from("/synthetic/batch.rs");
    let store = batch_store(&path, DIAGNOSTIC_COUNT);
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let run = |views: &mut Views, events| {
        panel_frame(&context, views, &slot, &store, &locale, &appearance, events)
    };
    run(&mut views, Vec::new());
    let wheel = |phase, delta| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        phase,
        delta,
        modifiers: egui::Modifiers::NONE,
    };
    let scroll = |views: &mut Views| {
        run(
            views,
            vec![
                egui::Event::PointerMoved(egui::pos2(WIDTH / 2.0, HEIGHT / 2.0)),
                wheel(egui::TouchPhase::Start, egui::Vec2::ZERO),
                wheel(egui::TouchPhase::Move, vec2(0.0, -SCROLL_DISTANCE)),
            ],
        );
        run(views, vec![wheel(egui::TouchPhase::End, egui::Vec2::ZERO)])
    };
    let (scrolled, _, original_id) = scroll(&mut views);
    let original_id = original_id.unwrap();
    assert!(
        egui::scroll_area::State::load(&context, original_id)
            .unwrap()
            .offset
            .y
            > 0.0
    );
    let filter = texts(&scrolled)
        .iter()
        .find(|(text, _)| text == &DIAGNOSTIC_COUNT.to_string())
        .unwrap()
        .1;
    for pressed in [true, false] {
        run(&mut views, pointer(filter, pressed));
    }
    let (filtered, _, _) = run(&mut views, Vec::new());
    let empty = crate::presentation::message(&locale, "problems.emptyFiltered", &[]);
    assert!(texts(&filtered).iter().any(|(text, _)| text == &empty));
    for pressed in [true, false] {
        run(&mut views, pointer(filter, pressed));
    }
    let (restored, _, restored_id) = run(&mut views, Vec::new());
    let restored_id = restored_id.unwrap();
    assert_eq!(
        egui::scroll_area::State::load(&context, restored_id)
            .unwrap()
            .offset
            .y,
        0.0
    );
    assert_ne!(original_id, restored_id);
    assert!(egui::scroll_area::State::load(&context, original_id).is_none());
    assert!(texts(&restored).iter().any(|(text, _)| text == "batch.rs"));
    let (scrolled, _, previous_id) = scroll(&mut views);
    let previous_id = previous_id.unwrap();
    assert!(
        egui::scroll_area::State::load(&context, previous_id)
            .unwrap()
            .offset
            .y
            > 0.0
    );
    let error = crate::presentation::message(&locale, "problems.severity.error", &[]);
    let target = scrolled
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(error.as_str()))
        .unwrap()
        .0;
    run(
        &mut views,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: target,
                data: None,
            },
        )],
    );
    run(
        &mut views,
        batch_keys(&[
            (egui::Key::Enter, true, false),
            (egui::Key::Enter, true, true),
        ]),
    );
    let (restored, _, next_id) = run(&mut views, batch_keys(&[(egui::Key::Enter, false, false)]));
    let next_id = next_id.unwrap();
    assert!(views.panels[&slot].active[diagnostics::ERROR]);
    assert_eq!(
        egui::scroll_area::State::load(&context, next_id)
            .unwrap()
            .offset
            .y,
        0.0
    );
    assert_ne!(next_id, previous_id);
    assert!(egui::scroll_area::State::load(&context, previous_id).is_none());
    assert!(texts(&restored).iter().any(|(text, _)| text == "batch.rs"));
    views.toggle(&slot);
    assert!(egui::scroll_area::State::load(&context, next_id).is_none());
}

#[test]
fn problems_키보드와_닫기는_반복_space_빈필터와_스크롤_재열기를_보존한다() {
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-problems-keyboard-{}", uuid::Uuid::new_v4())),
    ));
    let appearance = Appearance::new(
        &taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap(),
    )
    .unwrap();
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = editor
        .open_untitled(TabId::new(), "synthetic", "rust".into())
        .unwrap();
    let mut snapshot = editor.documents().snapshot(document).unwrap();
    let path = PathBuf::from("/synthetic/keyboard.rs");
    snapshot.key = DocumentKey::File(path.clone());
    let owner = diagnostics::Owner::new();
    let mut store = Store::default();
    store.retain_documents(HashSet::from([document]));
    store.reconcile(HashMap::from([(owner, HashSet::from([document]))]));
    store.publish(
        owner,
        &snapshot,
        (0..DIAGNOSTIC_COUNT)
            .map(|index| Diagnostic {
                message: format!("keyboard-diagnostic-{index}"),
                range: Range::new(
                    Position::new(index as u32, 0),
                    Position::new(index as u32, 1),
                ),
                ..Default::default()
            })
            .collect(),
    );
    let slot = ShellSlotId::new();
    let mut views = Views::new("en-US").unwrap();
    views.toggle(&slot);
    let context = egui::Context::default();
    context.enable_accesskit();
    let (output, _, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        Vec::new(),
    );
    assert!(
        texts(&output)
            .iter()
            .any(|(text, _)| text == "keyboard-diagnostic-0")
    );
    let group = Id::new(("problem-row", &slot, &path, None::<usize>));
    let key = |key, pressed, repeat| {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat,
            modifiers: egui::Modifiers::NONE,
        }]
    };
    context.memory_mut(|memory| memory.request_focus(group));
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Space, true, false),
    );
    assert_eq!(views.panels[&slot].rows.len(), 1);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Space, true, true),
    );
    assert_eq!(views.panels[&slot].rows.len(), 1);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Space, false, false),
    );
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, true, false),
    );
    assert_eq!(views.panels[&slot].rows.len(), DIAGNOSTIC_COUNT + 1);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, false, false),
    );
    let row = Id::new(("problem-row", &slot, &path, Some(0usize)));
    context.memory_mut(|memory| memory.request_focus(row));
    let (_, opened, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, true, false),
    );
    assert_eq!(opened.len(), 1);
    let opened = opened.into_iter().next().unwrap();
    assert_eq!(
        (&*opened.path, opened.line, opened.column),
        ("/synthetic/keyboard.rs", 1, 1)
    );
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, false, false),
    );
    let wheel = |phase, delta| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta,
        phase,
        modifiers: egui::Modifiers::NONE,
    };
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        vec![
            egui::Event::PointerMoved(egui::pos2(WIDTH / 2.0, HEIGHT / 2.0)),
            wheel(egui::TouchPhase::Start, egui::Vec2::ZERO),
            wheel(egui::TouchPhase::Move, vec2(0.0, -SCROLL_DISTANCE)),
        ],
    );
    let (scrolled, _, old_scroll) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        vec![wheel(egui::TouchPhase::End, egui::Vec2::ZERO)],
    );
    let old_scroll = old_scroll.unwrap();
    assert!(
        egui::scroll_area::State::load(&context, old_scroll)
            .unwrap()
            .offset
            .y
            > 0.0
    );
    assert!(
        !texts(&scrolled)
            .iter()
            .any(|(text, _)| text == "keyboard-diagnostic-0")
    );
    let close_label = crate::presentation::message(&locale, "common.close", &[]);
    let close = scrolled
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.label() == Some(close_label.as_str())).then(|| node.bounds().unwrap())
        })
        .unwrap();
    let close = egui::pos2(
        ((close.x0 + close.x1) / 2.0) as f32,
        ((close.y0 + close.y1) / 2.0) as f32,
    );
    for pressed in [true, false] {
        panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            pointer(close, pressed),
        );
    }
    assert!(!views.is_open(&slot));
    views.toggle(&slot);
    assert!(views.panels[&slot].viewport.is_none());
    let (reopened, _, new_scroll) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        Vec::new(),
    );
    let new_scroll = new_scroll.unwrap();
    assert_ne!(new_scroll, old_scroll);
    assert_eq!(
        egui::scroll_area::State::load(&context, new_scroll)
            .unwrap()
            .offset
            .y,
        0.0
    );
    assert!(
        texts(&reopened)
            .iter()
            .any(|(text, _)| text == "keyboard-diagnostic-0")
    );
    let filter = texts(&reopened)
        .iter()
        .find(|(text, _)| text == &DIAGNOSTIC_COUNT.to_string())
        .unwrap()
        .1;
    for pressed in [true, false] {
        panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            pointer(filter, pressed),
        );
    }
    let (filtered, _, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        Vec::new(),
    );
    let filtered_label = crate::presentation::message(&locale, "problems.emptyFiltered", &[]);
    assert!(
        texts(&filtered)
            .iter()
            .any(|(text, _)| text == &filtered_label)
    );
    assert_eq!(store.counts()[diagnostics::ERROR], DIAGNOSTIC_COUNT);
    let error_label = crate::presentation::message(&locale, "problems.severity.error", &[]);
    let filter_node = filtered
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(error_label.as_str()))
        .unwrap()
        .0;
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: filter_node,
                data: None,
            },
        )],
    );
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, true, false),
    );
    assert!(views.panels[&slot].active[diagnostics::ERROR]);
    assert_eq!(views.panels[&slot].rows.len(), DIAGNOSTIC_COUNT + 1);
}

#[test]
fn problems_헤더버튼은_space_해제와_enter_반복_포커스취소를_보존한다() {
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-problems-buttons-{}", uuid::Uuid::new_v4())),
    ));
    let appearance = Appearance::new(
        &taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap(),
    )
    .unwrap();
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    let context = egui::Context::default();
    context.enable_accesskit();
    let slot = ShellSlotId::new();
    let store = Store::default();
    let mut views = Views::new("en-US").unwrap();
    let key = |key, pressed, repeat| {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat,
            modifiers: egui::Modifiers::NONE,
        }]
    };
    let status = |views: &mut Views, events| {
        let mut response = None;
        let mut output = context.run_ui(input(events), |ui| {
            response = Some(
                views
                    .show_status(ui, Some(&slot), &store, &locale, &appearance)
                    .unwrap(),
            );
        });
        output.textures_delta.clear();
        response.unwrap()
    };
    let toggle = status(&mut views, Vec::new());
    context.memory_mut(|memory| memory.request_focus(toggle.id));
    status(&mut views, Vec::new());
    status(&mut views, key(egui::Key::Space, true, false));
    assert!(!views.is_open(&slot));
    status(&mut views, key(egui::Key::Space, true, true));
    assert!(!views.is_open(&slot));
    status(&mut views, key(egui::Key::Space, false, false));
    assert!(views.is_open(&slot));
    let focus = |output: &egui::FullOutput, label: &str| {
        let target = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label))
            .unwrap()
            .0;
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: target,
                data: None,
            },
        )]
    };
    let (output, _, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        Vec::new(),
    );
    let error = crate::presentation::message(&locale, "problems.severity.error", &[]);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        focus(&output, &error),
    );
    for repeat in [false, true] {
        panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            key(egui::Key::Space, true, repeat),
        );
        assert!(views.panels[&slot].active[diagnostics::ERROR]);
    }
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Space, false, false),
    );
    assert!(!views.panels[&slot].active[diagnostics::ERROR]);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, true, false),
    );
    assert!(views.panels[&slot].active[diagnostics::ERROR]);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, true, true),
    );
    assert!(!views.panels[&slot].active[diagnostics::ERROR]);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Enter, false, false),
    );
    let (output, _, _) = panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Space, true, false),
    );
    let close = crate::presentation::message(&locale, "common.close", &[]);
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        focus(&output, &close),
    );
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Space, false, false),
    );
    assert!(views.is_open(&slot));
    assert!(!views.panels[&slot].active[diagnostics::ERROR]);
    for repeat in [false, true] {
        panel_frame(
            &context,
            &mut views,
            &slot,
            &store,
            &locale,
            &appearance,
            key(egui::Key::Space, true, repeat),
        );
        assert!(views.is_open(&slot));
    }
    panel_frame(
        &context,
        &mut views,
        &slot,
        &store,
        &locale,
        &appearance,
        key(egui::Key::Space, false, false),
    );
    assert!(!views.is_open(&slot));
}

#[test]
fn problems_입력은_필터_가상행_접기_파일위치와_슬롯수명을_보존한다() {
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!(
            "taide-native-problems-{}",
            taide_model::ids::ProjectId::new()
        )),
    ));
    let theme =
        taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
    let appearance = Appearance::new(&theme).unwrap();
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let mut snapshots = Vec::new();
    for path in ["/synthetic/a.rs", "/synthetic/b.rs"] {
        let id = editor
            .open_untitled(TabId::new(), "synthetic", "rust".into())
            .unwrap();
        let mut snapshot = editor.documents().snapshot(id).unwrap();
        snapshot.key = DocumentKey::File(PathBuf::from(path));
        snapshots.push(snapshot);
    }
    let owner = diagnostics::Owner::new();
    let ids = snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    let mut store = Store::default();
    store.retain_documents(ids.clone());
    store.reconcile(HashMap::from([(owner, ids)]));
    store.publish(
        owner,
        &snapshots[0],
        (0..DIAGNOSTIC_COUNT)
            .map(|index| Diagnostic {
                message: format!("diagnostic-{index}"),
                range: Range::new(
                    Position::new(index as u32, 0),
                    Position::new(index as u32, 1),
                ),
                ..Default::default()
            })
            .collect(),
    );
    store.publish(
        owner,
        &snapshots[1],
        vec![Diagnostic {
            severity: Some(DiagnosticSeverity::WARNING),
            message: "warning-one".into(),
            source: Some("synthetic-server".into()),
            range: Range::new(Position::new(2, 7), Position::new(2, 8)),
            ..Default::default()
        }],
    );
    let mut views = Views::new("en-US").unwrap();
    let slot = ShellSlotId::new();
    let other = ShellSlotId::new();
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut response = None;
    let mut output = context.run_ui(input(Vec::new()), |ui| {
        response = Some(
            views
                .show_status(ui, Some(&slot), &store, &locale, &appearance)
                .unwrap(),
        );
    });
    let button = response.unwrap().rect.center();
    output.textures_delta.clear();
    for pressed in [true, false] {
        let mut output = context.run_ui(input(pointer(button, pressed)), |ui| {
            views
                .show_status(ui, Some(&slot), &store, &locale, &appearance)
                .unwrap();
        });
        output.textures_delta.clear();
    }
    assert!(views.is_open(&slot));
    assert!(!views.is_open(&other));
    let mut output = context.run_ui(input(Vec::new()), |ui| {
        views
            .show_panel(ui, &slot, &store, &locale, &appearance)
            .unwrap();
    });
    assert_eq!(views.panels[&slot].rows.len(), DIAGNOSTIC_COUNT + 3);
    let access = output.platform_output.accesskit_update.as_ref().unwrap();
    let error_label = crate::presentation::message(&locale, "problems.severity.error", &[]);
    let error_node = access
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label() == Some(error_label.as_str()))
        .unwrap();
    assert_eq!(error_node.role(), egui::accesskit::Role::Button);
    assert_eq!(error_node.toggled(), Some(egui::accesskit::Toggled::True));
    let group_label = crate::presentation::message(&locale, "problems.filterAriaLabel", &[]);
    assert!(
        access
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Group
                && node.label() == Some(group_label.as_str()))
    );
    let group_node = access
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label() == Some("/synthetic/a.rs"))
        .unwrap();
    assert_eq!(group_node.is_expanded(), Some(true));
    assert_eq!(
        views.panels[&slot].groups[0].path.as_ref(),
        &PathBuf::from("/synthetic/a.rs")
    );
    assert!(
        texts(&output)
            .iter()
            .filter(|(text, _)| text.starts_with("diagnostic-"))
            .count()
            < OVERSCAN * 2 + HEIGHT as usize / ROW_HEIGHT as usize
    );
    let filter = texts(&output)
        .into_iter()
        .find(|(text, _)| text == &DIAGNOSTIC_COUNT.to_string())
        .unwrap()
        .1;
    output.textures_delta.clear();
    for pressed in [true, false] {
        output = context.run_ui(input(pointer(filter, pressed)), |ui| {
            views
                .show_panel(ui, &slot, &store, &locale, &appearance)
                .unwrap();
        });
        output.textures_delta.clear();
    }
    assert!(!views.panels[&slot].active[diagnostics::ERROR]);
    assert_eq!(store.counts(), &[DIAGNOSTIC_COUNT, 1, 0, 0]);
    assert_eq!(views.panels[&slot].rows.len(), 2);
    output = context.run_ui(input(Vec::new()), |ui| {
        views
            .show_panel(ui, &slot, &store, &locale, &appearance)
            .unwrap();
    });
    let access = output.platform_output.accesskit_update.as_ref().unwrap();
    assert!(
        access
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some(error_label.as_str())
                && node.toggled() == Some(egui::accesskit::Toggled::False))
    );
    assert!(
        access.nodes.iter().any(|(_, node)| node
            .label()
            .is_some_and(|label| label.contains("warning-one")
                && label.contains("synthetic-server")
                && label.contains("3:8")))
    );
    let warning = texts(&output)
        .into_iter()
        .find(|(text, _)| text == "warning-one")
        .unwrap()
        .1;
    output.textures_delta.clear();
    let mut opened = Vec::new();
    for pressed in [true, false] {
        output = context.run_ui(input(pointer(warning, pressed)), |ui| {
            opened = views
                .show_panel(ui, &slot, &store, &locale, &appearance)
                .unwrap();
        });
        output.textures_delta.clear();
    }
    assert!(opened.len() <= 1);
    let opened = opened.pop().unwrap_or_else(|| {
        panic!(
            "warning position={warning:?}, text={:?}, row={:?}",
            texts(&output),
            context.read_response(Id::new((
                "problem-row",
                &slot,
                &PathBuf::from("/synthetic/b.rs"),
                Some(0usize)
            )))
        )
    });
    assert_eq!(
        (&*opened.path, opened.line, opened.column),
        ("/synthetic/b.rs", 3, 8)
    );
    let group = texts(&output)
        .into_iter()
        .find(|(text, _)| text == "b.rs")
        .unwrap()
        .1;
    for pressed in [true, false] {
        let mut output = context.run_ui(input(pointer(group, pressed)), |ui| {
            views
                .show_panel(ui, &slot, &store, &locale, &appearance)
                .unwrap();
        });
        output.textures_delta.clear();
    }
    assert_eq!(views.panels[&slot].rows.len(), 1);
    views.toggle(&slot);
    assert!(!views.is_open(&slot));
    views.toggle(&slot);
    assert_eq!(views.panels[&slot].active, [true; 4]);
    assert!(views.panels[&slot].collapsed.is_empty());
    views.toggle(&other);
    views.reconcile(Some(&ShellSlotTree::Leaf {
        slot_id: slot.clone(),
        project_id: taide_model::ids::ProjectId::new(),
    }));
    assert!(views.is_open(&slot));
    assert!(!views.is_open(&other));
    views.reconcile(None);
    assert!(views.panels.is_empty());
}
