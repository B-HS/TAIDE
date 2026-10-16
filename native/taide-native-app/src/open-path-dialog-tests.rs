use super::*;

const SCREEN: [f32; 2] = [800.0, 600.0];

fn locale() -> ResolvedLocale {
    ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    }
}

fn frame(
    context: &egui::Context,
    dialog: &mut OpenPathDialog,
    time: f64,
    events: Vec<egui::Event>,
) -> (Option<String>, egui::FullOutput) {
    let locale = locale();
    let mut result = None;
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(SCREEN[0], SCREEN[1]),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| result = dialog.show(ui.ctx(), &locale),
    );
    output.textures_delta.clear();
    (result, output)
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn 경로_열기는_입력을_비우고_trim과_enter와_대기중_중복과_실패유지_성공닫힘을_지킨다() {
    let context = egui::Context::default();
    let mut dialog = OpenPathDialog::default();
    assert!(frame(&context, &mut dialog, 0.0, Vec::new()).0.is_none());
    dialog.open();
    let _ = frame(&context, &mut dialog, 0.05, Vec::new());
    let (_, output) = frame(&context, &mut dialog, 0.1, Vec::new());
    let texts = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|text| text == "Open Project by Path"));
    assert!(
        texts
            .iter()
            .any(|text| text == "/path/to/project or ~/project")
    );
    assert!(context.memory(|memory| memory.has_focus(egui::Id::new(INPUT_ID))));
    assert!(
        frame(&context, &mut dialog, 0.2, vec![key(egui::Key::Enter)])
            .0
            .is_none(),
        "empty path does not confirm"
    );
    let _ = frame(&context, &mut dialog, 0.25, Vec::new());
    assert!(context.memory(|memory| memory.has_focus(egui::Id::new(INPUT_ID))));
    let (_, _) = frame(
        &context,
        &mut dialog,
        0.3,
        vec![egui::Event::Text("  ~/한글 project  ".into())],
    );
    let (result, _) = frame(&context, &mut dialog, 0.4, vec![key(egui::Key::Enter)]);
    assert_eq!(result.as_deref(), Some("~/한글 project"));
    let _ = frame(&context, &mut dialog, 0.5, Vec::new());
    assert!(context.memory(|memory| memory.has_focus(egui::Id::new(INPUT_ID))));
    assert!(
        frame(&context, &mut dialog, 0.6, vec![key(egui::Key::Enter)])
            .0
            .is_none(),
        "pending open is not submitted twice"
    );
    dialog.finish(false);
    assert!(dialog.is_open());
    assert_eq!(dialog.path, "  ~/한글 project  ");
    dialog.finish(true);
    assert!(!dialog.is_open());
    dialog.open();
    assert!(dialog.path.is_empty());
    let _ = frame(&context, &mut dialog, 1.0, Vec::new());
    let _ = frame(&context, &mut dialog, 1.1, vec![key(egui::Key::Escape)]);
    assert!(!dialog.is_open());
}
