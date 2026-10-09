use super::*;
use taide_model::{ids::ProjectId, paths::AppPaths};
use taide_native_editor::document_symbols::{Symbol, SymbolKind};
use taide_runtime::AppState;

const SCREEN: egui::Vec2 = vec2(360.0, 180.0);
const TIME_STEP: f64 = 0.05;
const GENERATION: u64 = 42;
const TREE_ID: &str = "symbol-outline-test";
const INPUT_ID: &str = "symbol-outline-external";

struct Harness {
    context: egui::Context,
    locale: ResolvedLocale,
    appearance: Appearance,
    icons: Icons,
    external: String,
    time: f64,
}

impl Harness {
    fn new() -> Self {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-outline-theme-{}", ProjectId::new())),
        ));
        let theme =
            taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
        let context = egui::Context::default();
        context.enable_accesskit();
        Self {
            context,
            locale: ResolvedLocale {
                id: "en".into(),
                name: "English".into(),
                warnings: Vec::new(),
                messages: serde_json::from_str(include_str!(
                    "../../../crates/taide-locale/resources/locales/en.json"
                ))
                .unwrap(),
            },
            appearance: Appearance::new(&theme).unwrap(),
            icons: Icons::new().unwrap(),
            external: String::new(),
            time: 0.0,
        }
    }

    fn frame(
        &mut self,
        panel: &mut Panel,
        scope: Scope<'_>,
        events: Vec<egui::Event>,
    ) -> (Output, egui::FullOutput) {
        self.time += TIME_STEP;
        let mut shown = None;
        let mut output = self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.external)
                            .id(Id::new(INPUT_ID))
                            .desired_width(100.0),
                    );
                    let _response = ui.button("next");
                });
                shown = Some(
                    panel
                        .show(
                            ui,
                            Id::new(TREE_ID),
                            Scope {
                                path: scope.path,
                                symbols: scope.symbols,
                            },
                            &self.locale,
                            &self.appearance,
                            &mut self.icons,
                        )
                        .unwrap(),
                );
            },
        );
        output.textures_delta.clear();
        (shown.unwrap(), output)
    }
}

fn symbols() -> Vec<Symbol> {
    [
        ("Outer", None, SymbolKind::CLASS),
        ("overload", Some(0), SymbolKind::FUNCTION),
        ("overload", Some(0), SymbolKind::METHOD),
        ("inner", Some(2), SymbolKind::VARIABLE),
        ("Next", None, SymbolKind::CONSTANT),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, parent, kind))| Symbol {
        name: name.into(),
        detail: format!("detail {index}"),
        kind,
        tags: Vec::new(),
        parent,
        container_label: String::new(),
        bytes: 0..1,
        selection: 0..1,
    })
    .collect()
}

fn scope(symbols: &[Symbol]) -> Scope<'_> {
    Scope {
        path: Some("/synthetic/current.rs"),
        symbols: SymbolIndex {
            entries: Some(symbols),
            generation: GENERATION,
            is_pending: false,
        },
    }
}

fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

fn texts(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn outline의_단일_focus와_트리_키_접기_클릭은_현재_세대_심볼만_reveal한다() {
    let symbols = symbols();
    let mut h = Harness::new();
    let mut panel = Panel::default();
    let (initial, painted) = h.frame(&mut panel, scope(&symbols), Vec::new());
    assert_eq!(initial.rows.len(), 5);
    assert!(texts(&painted).iter().any(|text| text == "Outer"));
    assert!(initial.rows.iter().all(
        |(_, response)| response.rect.height() == ROW_HEIGHT && !response.sense.is_focusable()
    ));
    h.context
        .memory_mut(|memory| memory.request_focus(Id::new(TREE_ID)));
    h.frame(&mut panel, scope(&symbols), Vec::new());
    let (selected, _) = h.frame(
        &mut panel,
        scope(&symbols),
        vec![key(Key::ArrowDown), key(Key::Enter)],
    );
    assert_eq!(selected.reveal, Some((GENERATION, 0)));
    h.frame(
        &mut panel,
        scope(&symbols),
        vec![
            key(Key::ArrowRight),
            key(Key::ArrowDown),
            key(Key::ArrowLeft),
        ],
    );
    assert_eq!(
        panel.rows.iter().map(|row| row.symbol).collect::<Vec<_>>(),
        [0, 1, 2, 4]
    );
    h.frame(
        &mut panel,
        scope(&symbols),
        vec![key(Key::ArrowRight), key(Key::ArrowRight)],
    );
    assert_eq!(panel.selected.as_deref(), Some("/0/1/0"));
    let (parent, _) = h.frame(
        &mut panel,
        scope(&symbols),
        vec![key(Key::ArrowLeft), key(Key::Enter)],
    );
    assert_eq!(parent.reveal, Some((GENERATION, 2)));
    let position =
        parent.rows[0].1.rect.left_center() + vec2(BASE_INDENT + CHEVRON_SIZE / 2.0, 0.0);
    let button = |pressed| egui::Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.frame(
        &mut panel,
        scope(&symbols),
        vec![egui::Event::PointerMoved(position), button(true)],
    );
    let (collapsed, _) = h.frame(&mut panel, scope(&symbols), vec![button(false)]);
    assert_eq!(collapsed.reveal, None);
    assert_eq!(panel.rows.len(), 2);
    assert_eq!(
        h.context.memory(|memory| memory.focused()),
        Some(Id::new(TREE_ID))
    );
    let (reset, _) = h.frame(
        &mut panel,
        Scope {
            path: Some("/synthetic/other.rs"),
            symbols: scope(&symbols).symbols,
        },
        Vec::new(),
    );
    assert_eq!(reset.reveal, None);
    assert_eq!(panel.rows.len(), 5);
    assert!(panel.collapsed.is_empty());
    assert_eq!(panel.selected, None);
}

#[test]
fn outline의_외부_입력_ime와_tab은_문자와_포커스를_탈취하지_않는다() {
    let symbols = symbols();
    let mut h = Harness::new();
    let mut panel = Panel::default();
    h.frame(&mut panel, scope(&symbols), Vec::new());
    h.context
        .memory_mut(|memory| memory.request_focus(Id::new(INPUT_ID)));
    h.frame(&mut panel, scope(&symbols), Vec::new());
    let (external, _) = h.frame(
        &mut panel,
        scope(&symbols),
        vec![
            egui::Event::Text("typed".into()),
            key(Key::ArrowDown),
            key(Key::Enter),
        ],
    );
    assert_eq!(h.external, "typed");
    assert_eq!(external.reveal, None);
    assert_eq!(panel.selected, None);
    h.context
        .memory_mut(|memory| memory.request_focus(Id::new(TREE_ID)));
    h.frame(&mut panel, scope(&symbols), Vec::new());
    h.frame(&mut panel, scope(&symbols), vec![key(Key::ArrowDown)]);
    h.frame(
        &mut panel,
        scope(&symbols),
        vec![
            egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "composing".into(),
                active_range_chars: None,
            }),
            key(Key::ArrowLeft),
        ],
    );
    h.frame(&mut panel, scope(&symbols), vec![key(Key::ArrowLeft)]);
    assert_eq!(panel.rows.len(), 5);
    h.frame(
        &mut panel,
        scope(&symbols),
        vec![
            egui::Event::Ime(egui::ImeEvent::Commit("done".into())),
            key(Key::ArrowLeft),
        ],
    );
    assert_eq!(panel.rows.len(), 5);
    h.frame(&mut panel, scope(&symbols), vec![key(Key::ArrowLeft)]);
    assert_eq!(panel.rows.len(), 2);
    h.frame(&mut panel, scope(&symbols), vec![key(Key::Tab)]);
    assert_ne!(
        h.context.memory(|memory| memory.focused()),
        Some(Id::new(TREE_ID))
    );
}

#[test]
fn outline은_대량_심볼을_가상화하고_마지막_키_선택과_빈_상태를_구별한다() {
    const COUNT: usize = 5000;
    const MAX_PAINTED: usize = 50;
    let symbols = (0..COUNT)
        .map(|index| Symbol {
            name: format!("item {index}"),
            detail: String::new(),
            kind: SymbolKind::STRING,
            tags: Vec::new(),
            parent: None,
            container_label: String::new(),
            bytes: 0..1,
            selection: 0..1,
        })
        .collect::<Vec<_>>();
    let mut h = Harness::new();
    let mut panel = Panel::default();
    let (initial, _) = h.frame(&mut panel, scope(&symbols), Vec::new());
    assert!(initial.rows.len() < MAX_PAINTED);
    h.context
        .memory_mut(|memory| memory.request_focus(Id::new(TREE_ID)));
    h.frame(&mut panel, scope(&symbols), Vec::new());
    let (last, _) = h.frame(
        &mut panel,
        scope(&symbols),
        vec![key(Key::ArrowUp), key(Key::Enter)],
    );
    assert_eq!(last.reveal, Some((GENERATION, COUNT - 1)));
    assert!(last.rows.iter().any(|(index, _)| *index == COUNT - 1));
    assert!(last.rows.len() < MAX_PAINTED);
    let (_, empty) = h.frame(
        &mut panel,
        Scope {
            path: Some("/synthetic/current.rs"),
            symbols: SymbolIndex::default(),
        },
        Vec::new(),
    );
    assert!(
        texts(&empty)
            .iter()
            .any(|text| text == "No symbols found in this file")
    );
    let (_, no_file) = h.frame(
        &mut panel,
        Scope {
            path: None,
            symbols: scope(&symbols).symbols,
        },
        Vec::new(),
    );
    assert!(texts(&no_file).iter().any(|text| text == "No file is open"));
}
