use super::*;
use taide_model::{ids::ProjectId, paths::AppPaths, tree::TreeRowPage};
use taide_runtime::AppState;

const SCREEN: egui::Vec2 = vec2(640.0, 480.0);
const SETTLED: f64 = 2.0;
const FOCUSED: f64 = 1.0;
const TOOLTIP_FONT: f32 = 12.0;
const NORMAL_TRIGGER: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(560.0, 100.0), egui::pos2(600.0, 140.0));
const HOVERED: f64 = 0.1;
const HOVER_OPENED: f64 = 0.5;
const CANCELLED: f64 = 0.532;

#[test]
fn explorer_입력의_escape는_일반_hover_tooltip이_열려있어도_오류를_취소한다() {
    use taide_model::tree::{TreeEntryKind, TreeRow};

    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-05-tooltip-escape-source.json"
    ))
    .unwrap();
    for case in source["results"].as_array().unwrap() {
        if case["mode"] == "modal" {
            continue;
        }
        assert_eq!(case["after"]["normalState"], "closed");
        assert_eq!(case["after"]["hasInput"], false);
        assert!(case["after"]["normalDescription"].is_null());
        let has_error = case["mode"] == "invalid";
        let locale = ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: serde_json::from_str(include_str!(
                "../../../crates/taide-locale/resources/locales/en.json"
            ))
            .unwrap(),
        };
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-explorer-mixed-escape-{}", ProjectId::new())),
        ));
        let theme =
            taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
        let appearance = crate::tooltips::Appearance::new(&theme).unwrap();
        let context = egui::Context::default();
        context.enable_accesskit();
        let provider = crate::tooltips::Provider::default();
        let project = ProjectId::new();
        let page = TreeRowPage {
            rows: vec![TreeRow {
                path: "/synthetic/file.txt".into(),
                name: "file.txt".into(),
                kind: TreeEntryKind::File,
                depth: 0,
                expanded: false,
                has_children: false,
            }],
            total: 1,
        };
        let mut explorer = crate::explorer::Explorer::default();
        explorer.root = Some("/synthetic".into());
        explorer.start_create(TreeEntryKind::File, &page);
        if has_error {
            explorer.create.as_mut().unwrap().input.name = "bad?".into();
            assert!(explorer.commit_create(&page.rows, &locale).is_none());
        } else {
            explorer.create.as_mut().unwrap().input.name = "valid.txt".into();
        }
        let normal = egui::Id::new("explorer-mixed-normal");
        let render = |explorer: &mut crate::explorer::Explorer, time, events| {
            let mut shown = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                    ..Default::default()
                },
                |ui| {
                    provider.begin_frame(ui.ctx());
                    let response = ui.interact(NORMAL_TRIGGER, normal, egui::Sense::click());
                    provider.show(
                        &response,
                        "Normal hover source",
                        egui::RectAlign::TOP,
                        &appearance,
                    );
                    let output = explorer.show(ui, &project, &page, &locale);
                    show_tooltips(&output, &locale, &provider, &appearance);
                    shown = Some(output);
                    provider.finish_frame(ui.ctx());
                },
            );
            output.textures_delta.clear();
            (output, shown.unwrap())
        };
        render(&mut explorer, 0.0, Vec::new());
        render(
            &mut explorer,
            HOVERED,
            vec![egui::Event::PointerMoved(NORMAL_TRIGGER.center())],
        );
        let (opened, shown) = render(&mut explorer, HOVER_OPENED, Vec::new());
        let input = shown.input.as_ref().unwrap();
        assert!(context.memory(|memory| memory.has_focus(input.id)));
        assert_eq!(shown.validation_error.is_some(), has_error);
        let nodes = &opened
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        assert_eq!(
            nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
                .count(),
            if has_error { 2 } else { 1 }
        );
        let (cancelled, shown) = render(
            &mut explorer,
            CANCELLED,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(explorer.create.is_none());
        assert!(shown.input.is_none() && shown.validation_error.is_none());
        let nodes = &cancelled
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
        assert_eq!(tooltips[0].1.label(), Some("Normal hover source"));
        let normal = &nodes
            .iter()
            .find(|(id, _)| *id == normal.accesskit_id())
            .unwrap()
            .1;
        assert!(
            normal.described_by().is_empty(),
            "document capture closes Tooltip before the input's stopPropagation and cancel"
        );
    }
}

#[test]
fn explorer_오류_tooltip은_create_rename의_controlled_open과_ax_invalid를_보존한다() {
    use taide_model::tree::{TreeEntryKind, TreeRow};

    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    let state = AppState::new(AppPaths::new(std::env::temp_dir().join(format!(
        "taide-explorer-validation-tooltip-{}",
        ProjectId::new()
    ))));
    let row = TreeRow {
        path: "/synthetic/file.txt".into(),
        name: "file.txt".into(),
        kind: TreeEntryKind::File,
        depth: 0,
        expanded: false,
        has_children: false,
    };
    let page = TreeRowPage {
        rows: vec![row.clone()],
        total: 1,
    };
    for name in ["vscode-dark-modern", "vscode-light-modern"] {
        let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
        let appearance = crate::tooltips::Appearance::new(&theme).unwrap();
        for rename in [false, true] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let tooltips = crate::tooltips::Provider::default();
            let project = ProjectId::new();
            let mut explorer = crate::explorer::Explorer::default();
            explorer.root = Some("/synthetic".into());
            if rename {
                explorer.start_rename(&row);
                explorer.rename.as_mut().unwrap().name = "bad?".into();
                assert!(explorer.commit_rename(&page.rows, &locale).is_none());
            } else {
                explorer.start_create(TreeEntryKind::File, &page);
                explorer.create.as_mut().unwrap().input.name = "bad?".into();
                assert!(explorer.commit_create(&page.rows, &locale).is_none());
            }
            let label = if rename {
                explorer.rename.as_ref().unwrap().error.clone()
            } else {
                explorer.create.as_ref().unwrap().input.error.clone()
            }
            .unwrap();
            let render = |explorer: &mut crate::explorer::Explorer, time, events| {
                let mut shown = None;
                let mut output = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        tooltips.begin_frame(&context);
                        let result = explorer.show(ui, &project, &page, &locale);
                        show_tooltips(&result, &locale, &tooltips, &appearance);
                        shown = Some(result);
                        tooltips.finish_frame(&context);
                    },
                );
                output.textures_delta.clear();
                (output, shown.unwrap())
            };
            render(&mut explorer, 0.0, Vec::new());
            let (output, shown) = render(&mut explorer, FOCUSED, Vec::new());
            let response = shown.input.as_ref().unwrap();
            let id = response.id.accesskit_id();
            let rect = response.rect;
            let nodes = &output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            let (tooltip_id, tooltip) = nodes
                .iter()
                .find(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
                .expect("controlled validation must expose Tooltip role");
            assert_eq!(tooltip.label(), Some(label.as_str()));
            let input = &nodes.iter().find(|(node, _)| *node == id).unwrap().1;
            assert_eq!(input.role(), egui::accesskit::Role::TextInput);
            assert_eq!(input.invalid(), Some(egui::accesskit::Invalid::True));
            assert_eq!(input.described_by(), &[*tooltip_id]);
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == crate::presentation::color(&theme, "tooltip.background").unwrap())));
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => Some(text),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                text.galley.job.sections[0].format.font_id,
                egui::FontId::proportional(TOOLTIP_FONT)
            );
            assert!(text.pos.y >= rect.bottom());
            let (away, _) = render(
                &mut explorer,
                SETTLED,
                vec![egui::Event::PointerMoved(SCREEN.to_pos2())],
            );
            assert!(
                away.platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
            );
            if rename {
                explorer.rename.as_mut().unwrap().error = None;
            } else {
                explorer.create.as_mut().unwrap().input.error = None;
            }
            let (closed, _) = render(&mut explorer, SETTLED, Vec::new());
            let nodes = &closed
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            assert!(
                !nodes
                    .iter()
                    .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
            );
            let input = &nodes.iter().find(|(node, _)| *node == id).unwrap().1;
            assert!(input.invalid().is_none());
            assert!(input.described_by().is_empty());
            if rename {
                explorer.rename.as_mut().unwrap().error = Some(label);
            } else {
                explorer.create.as_mut().unwrap().input.error = Some(label);
            }
            render(&mut explorer, SETTLED, Vec::new());
            let (cancelled, _) = render(
                &mut explorer,
                SETTLED,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(explorer.create.is_none() && explorer.rename.is_none());
            assert!(
                !cancelled
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
            );
        }
    }
}

#[test]
fn explorer_tooltip은_실제_4버튼의_bottom_테마와_ax_설명을_공유한다() {
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-explorer-tooltip-{}", ProjectId::new())),
    ));
    for name in ["vscode-dark-modern", "vscode-light-modern"] {
        let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
        let appearance = crate::tooltips::Appearance::new(&theme).unwrap();
        for key in KEYS {
            let context = egui::Context::default();
            context.enable_accesskit();
            let tooltips = crate::tooltips::Provider::default();
            let mut explorer = crate::explorer::Explorer::default();
            let project = ProjectId::new();
            let page = TreeRowPage {
                rows: Vec::new(),
                total: 0,
            };
            let mut render = |time, events| {
                let mut output = None;
                let mut painted = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        tooltips.begin_frame(&context);
                        let shown = explorer.show(ui, &project, &page, &locale);
                        show_tooltips(&shown, &locale, &tooltips, &appearance);
                        output = Some(shown);
                        tooltips.finish_frame(&context);
                    },
                );
                painted.textures_delta.clear();
                let output = output.unwrap();
                assert!(output.actions.is_empty());
                (painted, output)
            };
            let (_, layout) = render(0.0, Vec::new());
            let trigger = &layout.toolbar[key];
            let id = trigger.id;
            let rect = trigger.rect;
            render(
                FOCUSED,
                vec![egui::Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        target_node: id.accesskit_id(),
                        target_tree: egui::accesskit::TreeId::ROOT,
                        action: egui::accesskit::Action::Focus,
                        data: None,
                    },
                )],
            );
            render(FOCUSED, Vec::new());
            let (opened, _) = render(SETTLED, Vec::new());
            let label = crate::presentation::message(&locale, key, &[]);
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
            assert_eq!(tooltips.len(), 1, "{key}: {nodes:?}");
            let (tooltip_id, tooltip) = tooltips[0];
            assert_eq!(tooltip.label(), Some(label.as_str()));
            let trigger = &nodes
                .iter()
                .find(|(node, _)| *node == id.accesskit_id())
                .unwrap()
                .1;
            assert_eq!(trigger.role(), egui::accesskit::Role::Button);
            assert_eq!(trigger.described_by(), &[*tooltip_id]);
            assert!(opened.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == crate::presentation::color(&theme, "tooltip.background").unwrap())));
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
                egui::FontId::proportional(TOOLTIP_FONT)
            );
            assert!(text.pos.y >= rect.bottom());
            let (closed, _) = render(
                SETTLED,
                vec![egui::Event::Key {
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
        }
    }
}
