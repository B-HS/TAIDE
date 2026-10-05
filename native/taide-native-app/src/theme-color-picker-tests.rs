use super::*;
use taide_model::{ids::ProjectId, paths::AppPaths};
use taide_runtime::{AppState, locale_actions, theme_actions};

const SCREEN: [f32; 2] = [500.0, 500.0];
const FRAME_TIME: f64 = 0.1;

#[test]
fn native_theme_color_picker는_hex_hsv와_shared_color_범위를_보존한다() {
    for (input, expected) in [
        ("#abc", "#aabbcc"),
        ("#F00", "#ff0000"),
        ("#ABC123", "#abc123"),
        ("#11223300", "#112233"),
        (" #00ff00 ", "#00ff00"),
        ("#000000", "#000000"),
        ("#ffffff", "#ffffff"),
    ] {
        assert_eq!(Hsv::from_color(input).unwrap().hex(), expected);
        crate::presentation::parse_color(input, "fixture").unwrap();
    }
    for value in ["transparent", " TRANSPARENT "] {
        assert!(Hsv::from_color(value).is_none());
        assert_eq!(
            crate::presentation::parse_color(value, "fixture").unwrap(),
            Color32::TRANSPARENT
        );
    }
    for value in [
        "#abcd",
        "#12345",
        "#1234567",
        "#123456789",
        "#ＡＢＣ",
        "$palette",
        "red",
        "#ggg",
        "",
    ] {
        assert!(Hsv::from_color(value).is_none());
        assert!(crate::presentation::parse_color(value, "fixture").is_err());
    }
    assert_eq!(
        crate::presentation::parse_color("#abc", "fixture").unwrap(),
        Color32::from_rgb(170, 187, 204)
    );
    assert_eq!(
        crate::presentation::parse_color("#11223380", "fixture").unwrap(),
        Color32::from_rgba_unmultiplied(17, 34, 51, 128)
    );
    for (hue, color) in [
        (0.0, "#ff0000"),
        (60.0, "#ffff00"),
        (120.0, "#00ff00"),
        (180.0, "#00ffff"),
        (240.0, "#0000ff"),
        (300.0, "#ff00ff"),
        (360.0, "#ff0000"),
    ] {
        assert_eq!(
            Hsv {
                h: hue,
                s: 1.0,
                v: 1.0
            }
            .hex(),
            color
        );
    }
}

struct Fixture {
    context: egui::Context,
    picker: Picker,
    locale: ResolvedLocale,
    appearance: Appearance,
    value: String,
    time: f64,
    outside_focus: Option<Id>,
}

impl Fixture {
    fn new() -> Self {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-native-color-picker-{}", ProjectId::new())),
        ));
        let theme = theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
        let locale = locale_actions::locale_get_for_language(&state, "en", "en").unwrap();
        let mut picker = Picker::new("#ff0000".into());
        picker.open = true;
        Self {
            context: egui::Context::default(),
            picker,
            locale,
            appearance: Appearance::new(&theme).unwrap(),
            value: "#ff0000".into(),
            time: 0.0,
            outside_focus: None,
        }
    }

    fn frame(&mut self, events: Vec<egui::Event>) -> Vec<String> {
        self.time += FRAME_TIME;
        let mut changes = Vec::new();
        let mut output = self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                if let Some(id) = self.outside_focus.take() {
                    ui.memory_mut(|memory| memory.request_focus(id));
                }
                if let Some(value) = self.picker.show(
                    ui,
                    Id::new("synthetic-picker"),
                    &self.value,
                    &self.locale,
                    &self.appearance,
                ) {
                    changes.push(value);
                }
            },
        );
        output.textures_delta.clear();
        if let Some(value) = changes.last() {
            self.value = value.clone();
        }
        changes
    }

    fn settle(&mut self) {
        self.frame(Vec::new());
        self.frame(Vec::new());
    }
}

fn pointer(position: Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn key(key: Key) -> Vec<egui::Event> {
    vec![
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn native_theme_color_picker는_팝업_밖_클릭에도_hex_blur를_한번_적용한다() {
    const OUTSIDE_INSET: f32 = 5.0;
    let mut fixture = Fixture::new();
    fixture.settle();
    let input = fixture.picker.traces[3].unwrap().1.center();
    fixture.frame(pointer(input, true));
    fixture.frame(pointer(input, false));
    let mut select = key(Key::A);
    for event in &mut select {
        if let egui::Event::Key { modifiers, .. } = event {
            *modifiers = egui::Modifiers {
                command: true,
                mac_cmd: true,
                ..Default::default()
            };
        }
    }
    fixture.frame(select);
    fixture.frame(vec![egui::Event::Text("#244466".into())]);
    let outside = Pos2::new(SCREEN[0] - OUTSIDE_INSET, SCREEN[1] - OUTSIDE_INSET);
    fixture.outside_focus = Some(Id::new("synthetic-external-input"));
    let mut changes = fixture.frame(pointer(outside, true));
    changes.extend(fixture.frame(pointer(outside, false)));
    assert_eq!(changes, ["#244466"]);
    assert_eq!(fixture.value, "#244466");
    assert!(!fixture.picker.open);
    assert!(fixture.frame(Vec::new()).is_empty());
}

#[test]
fn native_theme_color_picker는_drag_단일commit과_keyboard_hex_blur를_연결한다() {
    let mut fixture = Fixture::new();
    fixture.settle();
    let square = fixture.picker.traces[1].unwrap().1;
    let sample = square.center();
    assert!(fixture.frame(pointer(sample, true)).is_empty());
    assert!(fixture.picker.drag.is_some());
    let moved = Pos2::new(square.right(), square.top());
    assert!(
        fixture
            .frame(vec![egui::Event::PointerMoved(moved)])
            .is_empty()
    );
    assert_eq!(fixture.value, "#ff0000");
    assert_eq!(fixture.frame(pointer(sample, false)), ["#804040"]);
    assert!(fixture.picker.drag.is_none());
    assert!(fixture.frame(pointer(sample, false)).is_empty());
    fixture.settle();
    assert_eq!(fixture.frame(key(Key::Home)), ["#000000"]);
    fixture.settle();
    assert_eq!(fixture.frame(key(Key::End)), ["#ffffff"]);
    let hue = fixture.picker.traces[2].unwrap().1;
    assert!(fixture.frame(pointer(hue.center(), true)).is_empty());
    assert!(fixture.frame(vec![egui::Event::PointerGone]).is_empty());
    assert!(fixture.picker.drag.is_none());
    assert!(fixture.frame(pointer(hue.center(), false)).is_empty());
    fixture.picker.open = false;
    fixture.settle();
    fixture.picker.open = true;
    fixture.settle();
    let input = fixture.picker.traces[3].unwrap().1.center();
    fixture.frame(pointer(input, true));
    fixture.frame(pointer(input, false));
    let mut select = key(Key::A);
    for event in &mut select {
        if let egui::Event::Key { modifiers, .. } = event {
            *modifiers = egui::Modifiers {
                command: true,
                mac_cmd: true,
                ..Default::default()
            };
        }
    }
    fixture.frame(select);
    fixture.frame(vec![egui::Event::Text("#AbC12380".into())]);
    assert!(fixture.frame(key(Key::Enter)).is_empty());
    assert_eq!(fixture.value, "#ffffff");
    let square = fixture.picker.traces[1].unwrap().1;
    assert_eq!(fixture.frame(pointer(square.center(), true)), ["#abc12380"]);
    assert!(!fixture.picker.hex_error);
    fixture.picker.open = false;
    fixture.frame(Vec::new());
    assert!(fixture.picker.drag.is_none());
}
