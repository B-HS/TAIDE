use super::*;

const WIDTH: f32 = 960.0;
const HEIGHT: f32 = 800.0;
const MEMORY: f64 = BYTES_PER_MEBIBYTE * 1.5;
const CPU: f64 = 42.5;

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

fn appearance() -> Appearance {
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-system-tooltip-{}", uuid::Uuid::new_v4())),
    ));
    let theme =
        taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
    Appearance {
        background: Color32::from_rgb(12, 20, 32),
        border: Color32::BLUE,
        foreground: Color32::WHITE,
        muted: Color32::GRAY,
        hover: Color32::DARK_GRAY,
        shadow: Color32::BLACK,
        focus: Color32::RED,
        tooltip: crate::tooltips::Appearance::new(&theme).unwrap(),
    }
}

fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(WIDTH, HEIGHT))),
        events,
        ..Default::default()
    }
}

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

fn texts(output: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some((text.galley.text().into(), text.pos)),
            _ => None,
        })
        .collect()
}

#[test]
fn system_usage_상태버튼과_상세modal은_원본_수치_종류순서_열_표시와_닫기를_그린다() {
    let context = egui::Context::default();
    let locale = locale();
    let appearance = appearance();
    let mut icon = Icon::new().unwrap();
    let tooltips = crate::tooltips::Provider::default();
    let usage = SystemUsage {
        cpu_percent: None,
        memory_bytes: MEMORY,
    };
    assert_eq!(summary_text(&locale, &usage), "CPU --% · RAM 2MB");
    assert_eq!(
        summary_text(
            &locale,
            &SystemUsage {
                cpu_percent: Some(CPU),
                ..usage.clone()
            }
        ),
        "CPU 43% · RAM 2MB"
    );
    let mut response = None;
    let mut output = context.run_ui(input(Vec::new()), |ui| {
        response =
            show_status(ui, &locale, &appearance, &mut icon, Some(&usage), &tooltips).unwrap();
    });
    assert!(
        texts(&output)
            .iter()
            .any(|(text, _)| text == "CPU --% · RAM 2MB")
    );
    assert!(icon.texture.is_some());
    assert!(output.textures_delta.set.iter().any(|(_, deltas)| {
        deltas
            .iter()
            .any(|delta| delta.image.width() == ICON_SIZE as usize)
    }));
    let position = response.as_ref().unwrap().rect.center();
    output.textures_delta.clear();
    let mut output = context.run_ui(input(pointer(position, true)), |ui| {
        response =
            show_status(ui, &locale, &appearance, &mut icon, Some(&usage), &tooltips).unwrap();
    });
    output.textures_delta.clear();
    let mut output = context.run_ui(input(pointer(position, false)), |ui| {
        response =
            show_status(ui, &locale, &appearance, &mut icon, Some(&usage), &tooltips).unwrap();
    });
    assert!(response.unwrap().clicked());
    output.textures_delta.clear();
    let mut output = context.run_ui(input(Vec::new()), |ui| {
        assert!(
            show_status(ui, &locale, &appearance, &mut icon, None, &tooltips)
                .unwrap()
                .is_none()
        );
    });
    assert!(
        !texts(&output)
            .iter()
            .any(|(text, _)| text.starts_with("CPU "))
    );
    output.textures_delta.clear();
    let rows = [
        (SystemUsageProcessKind::Other, "process-other"),
        (SystemUsageProcessKind::Agent, "process-agent"),
        (SystemUsageProcessKind::Lsp, "process-lsp"),
        (SystemUsageProcessKind::Terminal, "process-terminal"),
        (SystemUsageProcessKind::App, "process-app"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (kind, label))| SystemUsageProcess {
        pid: index as u32 + 1,
        kind,
        label: label.into(),
        cpu_percent: None,
        memory_bytes: MEMORY,
    })
    .collect::<Vec<_>>();
    for _ in 0..2 {
        let mut output = context.run_ui(input(Vec::new()), |_| {
            assert!(!show_detail(&context, &locale, &appearance, &rows));
        });
        output.textures_delta.clear();
    }
    let mut settled = input(Vec::new());
    settled.time = Some(1.0);
    let mut output = context.run_ui(settled, |_| {
        assert!(!show_detail(&context, &locale, &appearance, &rows));
    });
    let drawing = texts(&output);
    assert!(drawing.iter().any(|(text, _)| text == "Process Usage"));
    assert!(drawing.iter().any(|(text, _)| text == "PROCESS"));
    let positions = [
        "process-app",
        "process-terminal",
        "process-lsp",
        "process-agent",
        "process-other",
    ]
    .map(|label| drawing.iter().find(|(text, _)| text == label).unwrap().1);
    assert!(positions.windows(2).all(|pair| pair[0].y < pair[1].y));
    assert_eq!(
        drawing.iter().filter(|(text, _)| text == "0%").count(),
        GROUPS.len()
    );
    assert_eq!(
        drawing.iter().filter(|(text, _)| text == "2MB").count(),
        GROUPS.len()
    );
    for kind in ["APP", "TERMINAL", "LSP", "AGENT", "OTHER"] {
        assert!(drawing.iter().any(|(text, _)| text == kind));
    }
    let mut shapes = output
        .shapes
        .iter()
        .map(|shape| &shape.shape)
        .collect::<Vec<_>>();
    let mut modal = None;
    let mut close_position = None;
    while let Some(shape) = shapes.pop() {
        match shape {
            egui::Shape::Vec(children) => shapes.extend(children),
            egui::Shape::Rect(rect)
                if rect.fill == appearance.background && rect.stroke.color == appearance.border =>
            {
                modal = Some(rect.rect)
            }
            egui::Shape::LineSegment { points, stroke }
                if stroke.width == CLOSE_STROKE && stroke.color == appearance.muted =>
            {
                close_position = Some(points[0].lerp(points[1], 0.5))
            }
            _ => {}
        }
    }
    let modal = modal.unwrap();
    assert!((modal.width() - MAX_WIDTH).abs() <= BORDER);
    assert!((modal.height() - HEIGHT * HEIGHT_RATIO).abs() <= BORDER);
    output.textures_delta.clear();
    let close_position = close_position.unwrap();
    let mut output = context.run_ui(input(pointer(close_position, true)), |_| {
        assert!(!show_detail(&context, &locale, &appearance, &rows));
    });
    output.textures_delta.clear();
    let mut output = context.run_ui(input(pointer(close_position, false)), |_| {
        assert!(show_detail(&context, &locale, &appearance, &rows));
    });
    output.textures_delta.clear();
    let mut output = context.run_ui(input(Vec::new()), |_| {
        assert!(!show_detail(&context, &locale, &appearance, &[]));
    });
    assert!(
        texts(&output)
            .iter()
            .any(|(text, _)| text == "No processes to show")
    );
    output.textures_delta.clear();
    let mut output = context.run_ui(
        input(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]),
        |_| {
            assert!(show_detail(&context, &locale, &appearance, &rows));
        },
    );
    output.textures_delta.clear();
}
