use eframe::egui::{self, Color32, FontId, Id, Rect, Response, Stroke, Ui, vec2};
use taide_model::{
    error::AppResult,
    locale::ResolvedLocale,
    settings::{DEFAULT_EDITOR_FONT_SIZE, DEFAULT_TERMINAL_FONT_SIZE},
    theme::ResolvedTheme,
};
use taide_native_editor::{store::EditorStore, view::ViewKey};

use crate::{
    host::HostCommand,
    presentation::{color, message, next_editor_font_size},
};

const TEXT_SIZE: f32 = 11.0;
const ICON_SIZE: f32 = 12.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
const ICON_SLOT: f32 = 14.0;
const CONTROL_SIDE: f32 = 16.0;
const VALUE_WIDTH: f32 = 24.0;
const CONTROL_GAP: f32 = 2.0;
const STEPPER_WIDTH: f32 = ICON_SLOT + CONTROL_SIDE * 2.0 + VALUE_WIDTH + CONTROL_GAP * 3.0;
const CORNER_RADIUS: u8 = 2;
const TERMINAL_CORNER_RADIUS: u8 = 1;
const TYPE_CURVE_CONTROL: f32 = 0.552_284_8;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum Target {
    Editor,
    Terminal,
}

impl Target {
    fn label(self) -> &'static str {
        match self {
            Self::Editor => "window.editorFontSize",
            Self::Terminal => "window.terminalFontSize",
        }
    }

    fn command(self, value: u32) -> HostCommand {
        match self {
            Self::Editor => HostCommand::SetEditorFontSize(value),
            Self::Terminal => HostCommand::SetTerminalFontSize(value),
        }
    }

    fn default_size(self) -> u32 {
        match self {
            Self::Editor => DEFAULT_EDITOR_FONT_SIZE,
            Self::Terminal => DEFAULT_TERMINAL_FONT_SIZE,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Operation {
    Decrease,
    Reset,
    Increase,
}

impl Operation {
    fn label(self) -> &'static str {
        match self {
            Self::Decrease => "window.decreaseFontSize",
            Self::Reset => "window.resetFontSize",
            Self::Increase => "window.increaseFontSize",
        }
    }

    fn value(self, target: Target, current: u32) -> u32 {
        match self {
            Self::Decrease => next_editor_font_size(current, false),
            Self::Reset => target.default_size(),
            Self::Increase => next_editor_font_size(current, true),
        }
    }
}

pub(crate) struct Appearance {
    foreground: Color32,
    muted: Color32,
    hover: Color32,
    tooltip: crate::tooltips::Appearance,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            hover: color(theme, "appSidebar.itemHover")?,
            tooltip: crate::tooltips::Appearance::new(theme)?,
        })
    }
}

pub(crate) fn cursor(store: &EditorStore, key: Option<&ViewKey>) -> Option<(u32, u32)> {
    let view = store.views().get(store.views().find(key?)?)?;
    let head = view.selection.selections.get(view.selection.primary)?.head;
    let document = store.documents().snapshot(view.document).ok()?;
    let position = taide_native_editor::lsp::byte_to_position(&document, head).ok()?;
    Some((
        position.line.checked_add(1)?,
        position.character.checked_add(1)?,
    ))
}

pub(crate) fn show_cursor(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    position: Option<(u32, u32)>,
) {
    if let Some((line, column)) = position {
        ui.add(
            egui::Label::new(
                egui::RichText::new(message(
                    locale,
                    "editor.cursorPosition",
                    &[("line", &line.to_string()), ("column", &column.to_string())],
                ))
                .font(FontId::proportional(TEXT_SIZE))
                .color(appearance.muted),
            )
            .extend()
            .selectable(false),
        );
    }
}

fn control_id(context: &egui::Context, target: Target, operation: Operation) -> Id {
    Id::new((
        "native-status-font",
        context.viewport_id(),
        target,
        operation,
    ))
}

fn icon_id(context: &egui::Context, target: Target) -> Id {
    Id::new(("native-status-font-icon", context.viewport_id(), target))
}

fn paint_icon(ui: &Ui, rect: Rect, target: Target, color: Color32) {
    let scale = ICON_SIZE / ICON_VIEWBOX;
    let origin = rect.center() - vec2(ICON_SIZE, ICON_SIZE) / 2.0;
    let point = |[x, y]: [f32; 2]| origin + vec2(x, y) * scale;
    let stroke = Stroke::new(ICON_STROKE * scale, color);
    let line = |points: &[[f32; 2]]| {
        ui.painter().add(egui::Shape::line(
            points.iter().copied().map(point).collect(),
            stroke,
        ));
    };
    match target {
        Target::Editor => {
            line(&[[12.0, 4.0], [12.0, 20.0]]);
            line(&[[4.0, 7.0], [4.0, 5.0]]);
            ui.painter()
                .add(egui::epaint::CubicBezierShape::from_points_stroke(
                    [
                        [4.0, 5.0],
                        [4.0, 5.0 - TYPE_CURVE_CONTROL],
                        [5.0 - TYPE_CURVE_CONTROL, 4.0],
                        [5.0, 4.0],
                    ]
                    .map(point),
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                ));
            line(&[[5.0, 4.0], [19.0, 4.0]]);
            ui.painter()
                .add(egui::epaint::CubicBezierShape::from_points_stroke(
                    [
                        [19.0, 4.0],
                        [19.0 + TYPE_CURVE_CONTROL, 4.0],
                        [20.0, 5.0 - TYPE_CURVE_CONTROL],
                        [20.0, 5.0],
                    ]
                    .map(point),
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                ));
            line(&[[20.0, 5.0], [20.0, 7.0]]);
            line(&[[9.0, 20.0], [15.0, 20.0]]);
        }
        Target::Terminal => {
            line(&[[7.0, 11.0], [9.0, 9.0], [7.0, 7.0]]);
            line(&[[11.0, 13.0], [15.0, 13.0]]);
            ui.painter().rect_stroke(
                Rect::from_min_max(point([3.0, 3.0]), point([21.0, 21.0])),
                egui::CornerRadius::same(TERMINAL_CORNER_RADIUS),
                stroke,
                egui::StrokeKind::Middle,
            );
        }
    }
}

fn button(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    target: Target,
    operation: Operation,
    value: u32,
    tooltips: &crate::tooltips::Provider,
) -> Response {
    let label = message(
        locale,
        operation.label(),
        &[("label", &message(locale, target.label(), &[]))],
    );
    let width = if operation == Operation::Reset {
        VALUE_WIDTH
    } else {
        CONTROL_SIDE
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width, CONTROL_SIDE), egui::Sense::hover());
    let response = ui.interact(
        rect,
        control_id(ui.ctx(), target, operation),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    let highlighted = response.hovered() || response.has_focus();
    let foreground = if highlighted || operation == Operation::Reset {
        appearance.foreground
    } else {
        appearance.muted
    };
    if highlighted && operation != Operation::Reset {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(CORNER_RADIUS),
            appearance.hover,
        );
    }
    if operation == Operation::Reset {
        let text = ui.painter().layout_no_wrap(
            value.to_string(),
            FontId::proportional(TEXT_SIZE),
            foreground,
        );
        ui.painter()
            .galley(rect.center() - text.size() / 2.0, text, foreground);
    } else {
        let scale = ICON_SIZE / ICON_VIEWBOX;
        let origin = rect.center() - vec2(ICON_SIZE, ICON_SIZE) / 2.0;
        let point = |[x, y]: [f32; 2]| origin + vec2(x, y) * scale;
        let stroke = Stroke::new(ICON_STROKE * scale, foreground);
        ui.painter()
            .line_segment([point([5.0, 12.0]), point([19.0, 12.0])], stroke);
        if operation == Operation::Increase {
            ui.painter()
                .line_segment([point([12.0, 5.0]), point([12.0, 19.0])], stroke);
        }
    }
    tooltips.show(&response, &label, egui::RectAlign::TOP, &appearance.tooltip);
    response
}

pub(crate) fn show_font(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    target: Target,
    value: u32,
    commands: &mut Vec<HostCommand>,
    tooltips: &crate::tooltips::Provider,
) {
    ui.allocate_ui_with_layout(
        vec2(STEPPER_WIDTH, CONTROL_SIDE),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = CONTROL_GAP;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ICON_SLOT, ICON_SLOT), egui::Sense::hover());
            let response = ui.interact(rect, icon_id(ui.ctx(), target), egui::Sense::hover());
            paint_icon(ui, rect, target, appearance.muted);
            tooltips.show(
                &response,
                &message(locale, target.label(), &[]),
                egui::RectAlign::TOP,
                &appearance.tooltip,
            );
            for operation in [Operation::Decrease, Operation::Reset, Operation::Increase] {
                if button(ui, locale, appearance, target, operation, value, tooltips).clicked() {
                    commands.push(target.command(operation.value(target, value)));
                }
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
        time::Duration,
    };
    use taide_model::{
        app_event::AppEvent,
        ids::{PaneId, ProjectId, TabId},
        paths::AppPaths,
        settings::Settings,
    };
    use taide_native_editor::{
        store::EditorLimits,
        view::{Selection, SelectionSet},
    };
    use taide_runtime::{AppState, EventSink, TaskSupervisor};
    use tokio::sync::Notify;

    const SCREEN_WIDTH: f32 = 320.0;
    const SCREEN_HEIGHT: f32 = 24.0;
    const INITIAL_SIZE: u32 = 22;
    const MIN_SIZE: u32 = 6;
    const MAX_SIZE: u32 = 48;
    const DOCUMENT_BYTES: usize = 1024;
    const DEADLINE: Duration = Duration::from_secs(3);

    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Default)]
    struct Sink(Mutex<Vec<Settings>>);
    impl EventSink for Sink {
        fn publish(&self, event: AppEvent) {
            if let AppEvent::SettingsChanged { settings } = event {
                self.0.lock().unwrap().push(*settings);
            }
        }
    }

    #[test]
    fn 커서는_active_view의_primary_head와_utf16_행열을_사용한다() {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let key = ViewKey {
            window: "main".into(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let document = store
            .open_untitled(key.tab.clone(), "한😀\r\nx", "plaintext".into())
            .unwrap();
        assert_eq!(cursor(&store, Some(&key)), None);
        let view = store.attach_view(key.clone(), document).unwrap();
        assert_eq!(cursor(&store, Some(&key)), Some((1, 1)));
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 1,
                    selections: vec![
                        Selection { anchor: 0, head: 0 },
                        Selection {
                            anchor: 0,
                            head: "한😀".len(),
                        },
                    ],
                },
                Default::default(),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(cursor(&store, Some(&key)), Some((1, 4)));
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: 0,
                        head: "한😀\r\nx".len(),
                    }],
                },
                Default::default(),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(cursor(&store, Some(&key)), Some((2, 2)));
        let other = ViewKey {
            tab: TabId::new(),
            ..key.clone()
        };
        assert_eq!(cursor(&store, Some(&other)), None);
        assert_eq!(cursor(&store, None), None);
        store.detach_view(view).unwrap();
        assert_eq!(cursor(&store, Some(&key)), None);
        for target in [Target::Editor, Target::Terminal] {
            assert_eq!(Operation::Decrease.value(target, MIN_SIZE), MIN_SIZE);
            assert_eq!(Operation::Increase.value(target, MAX_SIZE), MAX_SIZE);
            assert_eq!(
                Operation::Reset.value(target, INITIAL_SIZE),
                DEFAULT_EDITOR_FONT_SIZE
            );
        }
    }

    fn frame(
        context: &egui::Context,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        settings: &Settings,
        events: Vec<egui::Event>,
        tooltips: &crate::tooltips::Provider,
    ) -> Vec<HostCommand> {
        let mut commands = Vec::new();
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                ui.horizontal(|ui| {
                    show_font(
                        ui,
                        locale,
                        appearance,
                        Target::Editor,
                        settings.editor_font_size,
                        &mut commands,
                        tooltips,
                    );
                    show_font(
                        ui,
                        locale,
                        appearance,
                        Target::Terminal,
                        settings.terminal_font_size,
                        &mut commands,
                        tooltips,
                    );
                });
            },
        );
        output.textures_delta.clear();
        commands
    }

    #[test]
    fn status_tooltip은_글꼴_8곳과_사용량의_테마_ax_수명을_공유한다() {
        const SCREEN: egui::Vec2 = vec2(640.0, 240.0);
        const CASE_INTERVAL: f64 = 4.0;
        const HOVER_WAIT: f64 = 1.0;
        const PAINT_SETTLE: f64 = 1.0;
        const TOOLTIP_FONT: f32 = 12.0;
        const PENDING_START: f64 = 0.1;
        const PENDING_DISABLED: f64 = 0.2;
        const PENDING_REENABLED: f64 = 0.3;
        const PENDING_EXPIRED: f64 = 0.8;
        let locale = ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: serde_json::from_str(include_str!(
                "../../../crates/taide-locale/resources/locales/en.json"
            ))
            .unwrap(),
        };
        for name in ["vscode-dark-modern", "vscode-light-modern"] {
            let state = AppState::new(AppPaths::new(
                std::env::temp_dir().join(format!("taide-status-tooltip-{}", ProjectId::new())),
            ));
            let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
            let appearance = Appearance::new(&theme).unwrap();
            let system_appearance = crate::system_usage_view::Appearance::new(&theme).unwrap();
            let mut icon = crate::system_usage_view::Icon::new().unwrap();
            let context = egui::Context::default();
            context.enable_accesskit();
            let tooltips = crate::tooltips::Provider::default();
            let usage = taide_model::system::SystemUsage {
                cpu_percent: None,
                memory_bytes: 0.0,
            };
            let mut render = |time, events, enabled| {
                let mut system = None;
                let mut commands = Vec::new();
                let mut output = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        tooltips.begin_frame(&context);
                        ui.add_space(SCREEN.y / 2.0);
                        ui.add_enabled_ui(enabled, |ui| {
                            ui.horizontal(|ui| {
                                for target in [Target::Editor, Target::Terminal] {
                                    show_font(
                                        ui,
                                        &locale,
                                        &appearance,
                                        target,
                                        INITIAL_SIZE,
                                        &mut commands,
                                        &tooltips,
                                    );
                                }
                                system = crate::system_usage_view::show_status(
                                    ui,
                                    &locale,
                                    &system_appearance,
                                    &mut icon,
                                    Some(&usage),
                                    &tooltips,
                                )
                                .unwrap();
                            });
                        });
                        tooltips.finish_frame(&context);
                    },
                );
                output.textures_delta.clear();
                assert!(commands.is_empty());
                (output, system.unwrap())
            };
            let (_, system) = render(0.0, Vec::new(), true);
            let pending_rect = context
                .read_response(icon_id(&context, Target::Editor))
                .unwrap()
                .rect;
            render(
                PENDING_START,
                vec![egui::Event::PointerMoved(pending_rect.center())],
                true,
            );
            render(PENDING_DISABLED, Vec::new(), false);
            render(PENDING_REENABLED, Vec::new(), true);
            let (expired, _) = render(PENDING_EXPIRED, Vec::new(), true);
            assert!(
                !expired
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
            );
            let mut targets = Vec::new();
            for target in [Target::Editor, Target::Terminal] {
                targets.push((
                    icon_id(&context, target),
                    message(&locale, target.label(), &[]),
                    false,
                ));
                for operation in [Operation::Decrease, Operation::Reset, Operation::Increase] {
                    targets.push((
                        control_id(&context, target, operation),
                        message(
                            &locale,
                            operation.label(),
                            &[("label", &message(&locale, target.label(), &[]))],
                        ),
                        true,
                    ));
                }
            }
            targets.push((
                system.id,
                message(&locale, "window.systemUsageHint", &[]),
                true,
            ));
            for (index, (id, label, focusable)) in targets.into_iter().enumerate() {
                let move_time = (index + 1) as f64 * CASE_INTERVAL;
                let ready_time = move_time + HOVER_WAIT;
                let paint_time = ready_time + PAINT_SETTLE;
                let rect = context.read_response(id).unwrap().rect;
                let events = if focusable {
                    vec![egui::Event::AccessKitActionRequest(
                        egui::accesskit::ActionRequest {
                            target_node: id.accesskit_id(),
                            target_tree: egui::accesskit::TreeId::ROOT,
                            action: egui::accesskit::Action::Focus,
                            data: None,
                        },
                    )]
                } else {
                    vec![egui::Event::PointerMoved(rect.center())]
                };
                render(move_time, events, true);
                render(ready_time, Vec::new(), true);
                let (opened, _) = render(paint_time, Vec::new(), true);
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
                assert_eq!(tooltips.len(), 1, "{label}: {nodes:?}");
                let (tooltip_id, tooltip) = tooltips[0];
                assert_eq!(tooltip.label(), Some(label.as_str()));
                let trigger = &nodes
                    .iter()
                    .find(|(node, _)| *node == id.accesskit_id())
                    .unwrap()
                    .1;
                assert_eq!(trigger.described_by(), &[*tooltip_id]);
                assert_eq!(
                    trigger.role(),
                    if focusable {
                        egui::accesskit::Role::Button
                    } else {
                        egui::accesskit::Role::GenericContainer
                    }
                );
                assert!(opened.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == color(&theme, "tooltip.background").unwrap())), "{label}");
                let text = opened
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => Some(text),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(
                    text.galley.job.sections[0].format.font_id,
                    FontId::proportional(TOOLTIP_FONT)
                );
                assert!(text.pos.y + text.galley.size().y <= rect.top());
                let (disabled, _) = render(paint_time, Vec::new(), false);
                assert!(
                    !disabled
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip),
                    "disabled {label}"
                );
                if focusable {
                    assert!(
                        disabled
                            .platform_output
                            .accesskit_update
                            .as_ref()
                            .unwrap()
                            .nodes
                            .iter()
                            .find(|(node, _)| *node == id.accesskit_id())
                            .unwrap()
                            .1
                            .described_by()
                            .is_empty()
                    );
                }
                render(paint_time, Vec::new(), true);
            }
        }
    }

    #[tokio::test]
    async fn 실제_글꼴_버튼은_설정_저장_이벤트를_거쳐_두_크기를_독립_변경한다() {
        let directory =
            Directory(std::env::temp_dir().join(format!("taide-status-font-{}", ProjectId::new())));
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        state.settings.write().editor_font_size = INITIAL_SIZE;
        state.settings.write().terminal_font_size = INITIAL_SIZE;
        let initial = state.settings.read().clone();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let events = Arc::new(Sink::default());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), events.clone());
        let owner = Arc::downgrade(&services);
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge = crate::host::HostBridge::connect_with_clipboard_ports(
            services,
            Arc::new(move || signal.notify_one()),
            Arc::new(|_| panic!("clipboard write is forbidden")),
            Arc::new(|| panic!("clipboard read is forbidden")),
            None,
        )
        .unwrap();
        let locale = ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: [
                ("window.editorFontSize".into(), "Editor Font Size".into()),
                (
                    "window.terminalFontSize".into(),
                    "Terminal Font Size".into(),
                ),
                (
                    "window.decreaseFontSize".into(),
                    "Decrease {{label}}".into(),
                ),
                ("window.resetFontSize".into(), "Reset {{label}}".into()),
                (
                    "window.increaseFontSize".into(),
                    "Increase {{label}}".into(),
                ),
            ]
            .into(),
        };
        let appearance = Appearance {
            foreground: Color32::WHITE,
            muted: Color32::GRAY,
            hover: Color32::DARK_GRAY,
            tooltip: crate::tooltips::Appearance::new(
                &taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into())
                    .unwrap(),
            )
            .unwrap(),
        };
        let context = egui::Context::default();
        let tooltips = crate::tooltips::Provider::default();
        assert!(
            frame(
                &context,
                &locale,
                &appearance,
                &initial,
                Vec::new(),
                &tooltips
            )
            .is_empty()
        );
        let mut expected = initial.clone();
        for target in [Target::Editor, Target::Terminal] {
            for operation in [Operation::Decrease, Operation::Reset, Operation::Increase] {
                let response = context
                    .read_response(control_id(&context, target, operation))
                    .unwrap();
                assert_eq!(response.rect.height(), CONTROL_SIDE);
                assert_eq!(
                    response.rect.width(),
                    if operation == Operation::Reset {
                        VALUE_WIDTH
                    } else {
                        CONTROL_SIDE
                    }
                );
                let point = response.rect.center();
                assert!(
                    frame(
                        &context,
                        &locale,
                        &appearance,
                        &expected,
                        vec![
                            egui::Event::PointerMoved(point),
                            egui::Event::PointerButton {
                                pos: point,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: Default::default()
                            }
                        ],
                        &tooltips,
                    )
                    .is_empty()
                );
                let mut commands = frame(
                    &context,
                    &locale,
                    &appearance,
                    &expected,
                    vec![egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    }],
                    &tooltips,
                );
                assert_eq!(commands.len(), 1);
                let current = match target {
                    Target::Editor => expected.editor_font_size,
                    Target::Terminal => expected.terminal_font_size,
                };
                let value = operation.value(target, current);
                let command = commands.pop().unwrap();
                match &command {
                    HostCommand::SetEditorFontSize(size) if target == Target::Editor => {
                        assert_eq!(*size, value)
                    }
                    HostCommand::SetTerminalFontSize(size) if target == Target::Terminal => {
                        assert_eq!(*size, value)
                    }
                    _ => panic!("wrong status font target"),
                }
                match target {
                    Target::Editor => expected.editor_font_size = value,
                    Target::Terminal => expected.terminal_font_size = value,
                }
                bridge.submit(command).unwrap();
                tokio::time::timeout(DEADLINE, ready.notified())
                    .await
                    .unwrap();
                assert!(bridge.poll().is_none());
                assert_eq!(*state.settings.read(), expected);
                let persisted: Settings =
                    serde_json::from_slice(&std::fs::read(state.paths.settings_file()).unwrap())
                        .unwrap();
                assert_eq!(persisted, expected);
                assert_eq!(events.0.lock().unwrap().last(), Some(&expected));
            }
        }
        assert_eq!(events.0.lock().unwrap().len(), 6);
        bridge.disconnect().await.unwrap();
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
        assert!(owner.upgrade().is_none());
    }
}
