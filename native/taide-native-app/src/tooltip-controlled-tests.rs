use super::*;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const NORMAL: Rect = Rect::from_min_max(egui::pos2(80.0, 80.0), egui::pos2(104.0, 104.0));
const INPUT: Rect = Rect::from_min_max(egui::pos2(240.0, 200.0), egui::pos2(360.0, 224.0));
const OPENED: f64 = 0.4;
const FOCUSED: f64 = 0.5;
const CLOSE_STARTED: f64 = 0.6;
const CLOSE_REPEATED: f64 = 0.7;
const CLEARED: f64 = 0.8;
const EXPIRED: f64 = 1.0;
const OUTSIDE: Pos2 = egui::pos2(560.0, 440.0);

#[test]
fn controlled_tooltip은_prop_변경과_닫기_시도를_분리하고_skip_타이머를_보존한다() {
    let context = Context::default();
    context.enable_accesskit();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("controlled-requests");
    let render = |time, error: Option<&str>, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                let response = ui.interact(INPUT, id, egui::Sense::click());
                response.widget_info(|| egui::WidgetInfo::text_edit(true, "", "", ""));
                provider.show_controlled(&response, error, egui::RectAlign::BOTTOM, &appearance);
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        output
    };
    let snapshot = || {
        let owners = provider.0.lock().unwrap();
        let viewport = owners.get(&ViewportId::ROOT).unwrap();
        (
            viewport.is_delayed,
            viewport.skip_until,
            viewport.open,
            viewport.widgets.get(&id).unwrap().controlled,
            viewport.pending.len(),
        )
    };
    render(0.0, None, Vec::new());
    context.memory_mut(|memory| memory.request_focus(id));
    render(OPENED, None, Vec::new());
    assert_eq!(snapshot(), (false, None, None, Some(false), 0));
    render(FOCUSED, Some("Validation error"), Vec::new());
    assert_eq!(
        snapshot(),
        (false, None, None, Some(true), 0),
        "external prop must not emit onChange"
    );
    let enter = Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    render(FOCUSED, Some("Validation error"), vec![enter]);
    assert!(
        snapshot().1.is_none(),
        "input Enter is not a Tooltip trigger click"
    );
    let press = || Event::PointerButton {
        pos: OUTSIDE,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    };
    let output = render(
        CLOSE_STARTED,
        Some("Validation error"),
        vec![Event::PointerMoved(OUTSIDE), press()],
    );
    assert_eq!(
        snapshot(),
        (
            false,
            Some(CLOSE_STARTED + SKIP_DELAY_SECONDS),
            None,
            Some(true),
            0
        )
    );
    assert!(
        output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip
                && node.label() == Some("Validation error"))
    );
    render(CLOSE_REPEATED, Some("Validation error"), vec![press()]);
    let deadline = CLOSE_REPEATED + SKIP_DELAY_SECONDS;
    assert_eq!(snapshot().1, Some(deadline));
    let output = render(CLEARED, None, Vec::new());
    assert_eq!(snapshot(), (false, Some(deadline), None, Some(false), 0));
    assert!(
        !output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
    );
    render(EXPIRED, None, Vec::new());
    assert_eq!(snapshot(), (true, None, None, Some(false), 0));
}

#[test]
fn controlled_tooltip은_초기_오류의_닫기시도로_기본_지연을_건너뛰지_않는다() {
    let context = Context::default();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let id = Id::new("controlled-initial-error");
    let mut output = context.run_ui(
        egui::RawInput {
            time: Some(0.0),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| {
            provider.begin_frame(ui.ctx());
            let response = ui.interact(INPUT, id, egui::Sense::click());
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
    let mut output = context.run_ui(
        egui::RawInput {
            time: Some(OPENED),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            events: vec![Event::PointerButton {
                pos: OUTSIDE,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| {
            provider.begin_frame(ui.ctx());
            let response = ui.interact(INPUT, id, egui::Sense::click());
            provider.show_controlled(
                &response,
                Some("Validation error"),
                egui::RectAlign::BOTTOM,
                &appearance,
            );
            let response = ui.interact(
                NORMAL,
                Id::new("normal-after-initial-error"),
                egui::Sense::click(),
            );
            assert!(!provider.is_open(&response));
            provider.finish_frame(ui.ctx());
        },
    );
    output.textures_delta.clear();
    let owners = provider.0.lock().unwrap();
    let viewport = owners.get(&ViewportId::ROOT).unwrap();
    assert!(viewport.is_delayed);
    assert_eq!(viewport.skip_until, Some(OPENED + SKIP_DELAY_SECONDS));
}

#[test]
fn controlled_tooltip은_오류없는_input의_focus_열기시도로_기존_tooltip을_닫는다() {
    let context = Context::default();
    context.enable_accesskit();
    let provider = Provider::default();
    let appearance = Appearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
    };
    let normal = Id::new("controlled-graph-normal");
    let input = Id::new("controlled-graph-input");
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: Default::default(),
    };
    let render = |time, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                provider.begin_frame(ui.ctx());
                let response = ui.interact(NORMAL, normal, egui::Sense::click());
                provider.show(
                    &response,
                    "Normal tooltip",
                    egui::RectAlign::TOP,
                    &appearance,
                );
                let response = ui.interact(INPUT, input, egui::Sense::click());
                response.widget_info(|| egui::WidgetInfo::text_edit(true, "", "", ""));
                crate::explorer_toolbar::show_tooltips(
                    &crate::explorer::Output {
                        actions: Vec::new(),
                        rows: HashMap::new(),
                        icons: HashMap::new(),
                        draft_icon: None,
                        icon_error: None,
                        input: Some(response),
                        validation_error: None,
                        toolbar: HashMap::new(),
                        menu: HashMap::new(),
                        blank: None,
                    },
                    &locale,
                    &provider,
                    &appearance,
                );
                provider.finish_frame(ui.ctx());
            },
        );
        output.textures_delta.clear();
        output
    };
    render(0.0, Vec::new());
    render(0.0, vec![Event::PointerMoved(NORMAL.center())]);
    render(OPENED, Vec::new());
    assert_eq!(
        provider
            .0
            .lock()
            .unwrap()
            .get(&ViewportId::ROOT)
            .unwrap()
            .open,
        Some(normal)
    );
    context.memory_mut(|memory| memory.request_focus(input));
    let output = render(FOCUSED, Vec::new());
    assert!(
        provider
            .0
            .lock()
            .unwrap()
            .get(&ViewportId::ROOT)
            .unwrap()
            .open
            .is_none(),
        "controlled open=false still emits a global open attempt on focus"
    );
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let node = &nodes
        .iter()
        .find(|(id, _)| *id == input.accesskit_id())
        .unwrap()
        .1;
    assert!(node.described_by().is_empty());
    let normal_node = &nodes
        .iter()
        .find(|(id, _)| *id == normal.accesskit_id())
        .unwrap()
        .1;
    assert!(
        normal_node.described_by().is_empty(),
        "global close must update the already-registered trigger in the same pass"
    );
    assert!(
        !nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip
                && node.label() == Some("Validation error"))
    );
}
