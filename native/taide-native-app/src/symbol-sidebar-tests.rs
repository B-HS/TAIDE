use super::*;
use taide_model::{ids::PaneId, layout::AuxWindowLayout, paths::AppPaths};

#[test]
fn outline_진입은_슬롯과_프로젝트별로_보존하고_교체와_닫힘에서_폐기한다() {
    let first = ProjectId::new();
    let second = ProjectId::new();
    let left = ShellSlotId::new();
    let right = ShellSlotId::new();
    let mut views = Views::default();
    views.entry(&first, &left).view = View::Outline;
    views.entry(&second, &right).view = View::Outline;
    assert_eq!(views.entry(&first, &left).view, View::Outline);
    assert_eq!(views.entry(&second, &left).view, View::Files);
    let tree = ShellSlotTree::Leaf {
        slot_id: right.clone(),
        project_id: second.clone(),
    };
    views.reconcile(Some(&tree));
    assert_eq!(views.entries.len(), 1);
    assert_eq!(views.entry(&second, &right).view, View::Outline);
    views.reconcile(None);
    assert!(views.entries.is_empty());
    assert_eq!(views.entry(&second, &right).view, View::Files);
}

#[test]
fn outline_집중_트리는_현재_프로젝트와_보조_창_slot의_pane만_사용한다() {
    const AUX_SLOT: u32 = 4;
    let project = ProjectId::new();
    let mut layout = taide_layout::service::default_layout();
    let auxiliary = PaneId::new();
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: AUX_SLOT,
        root: PaneNode::Leaf {
            id: auxiliary.clone(),
            tabs: Vec::new(),
            active: None,
        },
        focused_pane: auxiliary.clone(),
    });
    assert_eq!(
        window_tree(&project, &layout, &WindowScope::Main)
            .unwrap()
            .1,
        &layout.focused_pane
    );
    let scope = WindowScope::Auxiliary {
        project: project.clone(),
        slot: AUX_SLOT,
    };
    assert_eq!(
        window_tree(&project, &layout, &scope).unwrap().1,
        &auxiliary
    );
    assert!(window_tree(&ProjectId::new(), &layout, &scope).is_none());
    layout.auxiliary_windows.clear();
    assert!(window_tree(&project, &layout, &scope).is_none());
}

#[test]
fn 파일과_아웃라인_진입_버튼은_실제_높이와_focus와_클릭_선택을_보존한다() {
    const SCREEN_WIDTH: f32 = 300.0;
    const SCREEN_HEIGHT: f32 = 140.0;
    let state = taide_runtime::AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-sidebar-theme-{}", ProjectId::new())),
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
    let ctx = egui::Context::default();
    let mut icons = Icons::new().unwrap();
    let mut views = Views::default();
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let mut responses = Vec::new();
    let mut events = Vec::new();
    for frame in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
                )),
                events: std::mem::take(&mut events),
                ..Default::default()
            },
            |ui| {
                responses = views
                    .entry(&project, &slot)
                    .switch(
                        ui,
                        Id::new("sidebar-test"),
                        &locale,
                        &appearance,
                        &mut icons,
                    )
                    .unwrap();
            },
        );
        output.textures_delta.clear();
        if frame == 2 {
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(painted) if painted.rect == responses[1].1.rect && painted.fill == appearance.selected)), "클릭한 프레임에 아웃라인이 선택색이어야 한다");
        }
        assert_eq!(responses.len(), 2);
        assert!(responses.iter().all(|(_, response)| response.rect.size()
            == vec2(BUTTON_SIZE, BUTTON_SIZE)
            && response.sense.is_focusable()));
        if frame == 0 {
            let pos = responses[1].1.rect.center();
            events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
        }
        if frame == 1 {
            let pos = responses[1].1.rect.center();
            events = vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }];
        }
    }
    assert_eq!(views.entry(&project, &slot).view, View::Outline);
    assert_eq!(
        ctx.memory(|memory| memory.focused()),
        Some(responses[1].1.id)
    );
}
