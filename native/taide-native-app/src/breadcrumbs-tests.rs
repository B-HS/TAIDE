use super::*;
use taide_model::{
    file::{FileSizeTier, OpenedFile},
    layout::{AuxWindowLayout, PaneNode, Tab},
    paths::AppPaths,
};
use taide_native_editor::{
    document_symbols::{Symbol, SymbolKind},
    store::{EditorLimits, EditorStore},
};

const PATH: &str = "/project/src/current.rs";
const ROOT: &str = "/project";
const GENERATION: u64 = 41;
const MAX_BYTES: usize = 4096;
const CONTENT_BYTES: usize = 128;
const SCREEN: egui::Vec2 = vec2(800.0, 350.0);
const EXTERNAL_ID: &str = "breadcrumb-external";
const BAR_ID: &str = "breadcrumb-test";
const TIME_STEP: f64 = 0.05;
const TYPEAHEAD_DELAY: f64 = 1.1;

fn document() -> (EditorStore, DocumentSnapshot, Source) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: 2,
        max_document_bytes: MAX_BYTES,
    })
    .unwrap();
    let document = store
        .open_file(
            PATH.into(),
            OpenedFile {
                path: PATH.into(),
                content: "x".repeat(CONTENT_BYTES),
                byte_size: CONTENT_BYTES.try_into().unwrap(),
                line_count: 1,
                language_id: "rust".into(),
                tier: FileSizeTier::Normal,
                read_only: true,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: Default::default(),
            },
        )
        .unwrap();
    let snapshot = store.documents().snapshot(document).unwrap();
    let source = Source {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
        path: PATH.into(),
        document,
        revision: snapshot.revision,
        language: "rust".into(),
        generation: GENERATION,
    };
    (store, snapshot, source)
}

fn layout(source: &Source) -> ProjectLayout {
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = source.pane.clone();
    layout.root = PaneNode::Leaf {
        id: source.pane.clone(),
        tabs: vec![Tab {
            id: source.tab.clone(),
            kind: TabKind::File {
                path: source.path.clone(),
            },
            title: "current.rs".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(source.tab.clone()),
    };
    layout
}

fn symbols() -> Vec<Symbol> {
    [
        ("Outer", None, 0..90),
        ("first", Some(0), 10..35),
        ("second", Some(0), 40..75),
        ("only", Some(1), 20..25),
        ("Next", None, 100..120),
    ]
    .into_iter()
    .map(|(name, parent, bytes)| Symbol {
        name: name.into(),
        detail: String::new(),
        kind: SymbolKind::FUNCTION,
        tags: Vec::new(),
        parent,
        container_label: String::new(),
        selection: bytes.clone(),
        bytes,
    })
    .collect()
}

fn rows() -> Vec<TreeRow> {
    [
        ("src", "/project/src", TreeEntryKind::Directory),
        ("other.rs", "/project/src/other.rs", TreeEntryKind::File),
        ("current.rs", PATH, TreeEntryKind::File),
        ("nested", "/project/src/nested", TreeEntryKind::Directory),
        (
            "child.rs",
            "/project/src/nested/child.rs",
            TreeEntryKind::File,
        ),
    ]
    .into_iter()
    .map(|(name, path, kind)| TreeRow {
        name: name.into(),
        path: path.into(),
        kind,
        depth: 0,
        expanded: false,
        has_children: false,
    })
    .collect()
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

struct Harness {
    ctx: egui::Context,
    locale: ResolvedLocale,
    appearance: Appearance,
    icons: Icons,
    external: String,
    time: f64,
}

impl Harness {
    fn new() -> Self {
        let state = taide_runtime::AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-breadcrumb-theme-{}", ProjectId::new())),
        ));
        let theme =
            taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        Self {
            ctx,
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
        bar: &mut Bar,
        scope: Scope<'_>,
        events: Vec<egui::Event>,
    ) -> (Output, egui::FullOutput) {
        self.time += TIME_STEP;
        let mut shown = None;
        let mut output = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events,
                time: Some(self.time),
                ..Default::default()
            },
            |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.external).id(Id::new(EXTERNAL_ID)));
                shown = Some(
                    bar.show(
                        ui,
                        Id::new(BAR_ID),
                        Scope {
                            source: scope.source.clone(),
                            root: scope.root,
                            caret: scope.caret,
                            symbols: scope.symbols,
                            rows: scope.rows,
                        },
                        &self.locale,
                        &self.appearance,
                        &mut self.icons,
                    )
                    .unwrap(),
                );
                let _response = ui.button("editor placeholder");
            },
        );
        output.textures_delta.clear();
        (shown.unwrap(), output)
    }
}

fn scope<'a>(
    source: &Source,
    symbols: &'a [Symbol],
    rows: &'a [TreeRow],
    caret: usize,
) -> Scope<'a> {
    Scope {
        source: Some(source.clone()),
        root: ROOT,
        caret: Some(caret),
        symbols: SymbolIndex {
            entries: Some(symbols),
            generation: source.generation,
            is_pending: false,
        },
        rows,
    }
}

#[test]
fn breadcrumb_원본은_문서_버전_언어_세대와_주창_보조창의_활성_탭을_검증한다() {
    const AUX_SLOT: u32 = 3;
    let (_, snapshot, source) = document();
    assert!(source.describes(&snapshot, GENERATION));
    assert!(!source.describes(&snapshot, GENERATION + 1));
    let mut changed = snapshot.clone();
    changed.revision += 1;
    assert!(!source.describes(&changed, GENERATION));
    changed = snapshot.clone();
    changed.metadata.language_id = "typescript".into();
    assert!(!source.describes(&changed, GENERATION));
    let mut layout = layout(&source);
    assert!(source.is_active(&layout));
    let PaneNode::Leaf { active, .. } = &mut layout.root else {
        panic!("leaf")
    };
    *active = None;
    assert!(!source.is_active(&layout));
    let root = crate::breadcrumbs::tests::layout(&source).root;
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: AUX_SLOT,
        root,
        focused_pane: source.pane.clone(),
    });
    assert!(source.is_active(&layout));
    layout.auxiliary_windows.clear();
    assert!(!source.is_active(&layout));
}

#[test]
fn 경로와_현재_caret_심볼은_한줄_32px_막대와_실제_수준_형제_메뉴를_사용한다() {
    let (_, _, source) = document();
    let symbols = symbols();
    let rows = rows();
    let mut h = Harness::new();
    let mut bar = Bar::default();
    let (initial, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    assert_eq!(initial.segments.len(), 5);
    assert!(
        initial
            .segments
            .iter()
            .all(|response| response.rect.height() < HEIGHT)
    );
    assert!(!initial.segments[4].sense.is_focusable());
    let trigger = initial.segments[3].id;
    h.ctx.memory_mut(|memory| memory.request_focus(trigger));
    h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    let (opened, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::Enter)],
    );
    assert_eq!(opened.entries.len(), 2);
    assert!(opened.actions.is_empty());
    let (visible, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    assert!(visible.entries.iter().all(Response::enabled));
    assert_eq!(
        h.ctx.memory(|memory| memory.focused()),
        Some(visible.entries[0].id)
    );
    let (moved, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::ArrowDown)],
    );
    assert_eq!(
        h.ctx.memory(|memory| memory.focused()),
        Some(opened.entries[1].id),
        "popup={} rows={:?} action-indices={:?}",
        Popup::is_any_open(&h.ctx),
        moved
            .entries
            .iter()
            .map(|row| (row.id, row.rect, row.clicked(), row.has_focus()))
            .collect::<Vec<_>>(),
        moved
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::RevealSymbol { index, .. } => Some(*index),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
    let (selected, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::Enter)],
    );
    assert!(
        matches!(selected.actions.as_slice(), [Action::RevealSymbol { source: selected_source, index: 2 }] if selected_source == &source)
    );
    assert!(!Popup::is_any_open(&h.ctx));
}

#[test]
fn 경로_메뉴는_tree를_준비하고_직계_파일만_열며_directory와_외부입력_ime를_보존한다() {
    let (_, _, source) = document();
    let symbols = symbols();
    let rows = rows();
    let mut h = Harness::new();
    let mut bar = Bar::default();
    let (initial, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    h.ctx
        .memory_mut(|memory| memory.request_focus(initial.segments[1].id));
    h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    let (opened, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::Enter)],
    );
    assert!(
        matches!(opened.actions.as_slice(), [Action::RevealTree(selected)] if selected == &source)
    );
    assert_eq!(opened.entries.len(), 3);
    assert!(!opened.entries[2].sense.is_focusable());
    h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    let focused = h.ctx.memory(|memory| memory.focused());
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "조합".into(),
            active_range_chars: None,
        })],
    );
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::ArrowDown)],
    );
    assert_eq!(h.ctx.memory(|memory| memory.focused()), focused);
    let (commit, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![
            egui::Event::Ime(egui::ImeEvent::Commit("완료".into())),
            key(egui::Key::Enter),
        ],
    );
    assert!(commit.actions.is_empty());
    h.ctx
        .memory_mut(|memory| memory.request_focus(Id::new(EXTERNAL_ID)));
    let (external, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![egui::Event::Text("external".into()), key(egui::Key::Enter)],
    );
    assert!(external.actions.is_empty());
    assert_eq!(h.external, "external");
    h.ctx
        .memory_mut(|memory| memory.request_focus(opened.entries[0].id));
    h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    let (selected, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::Enter)],
    );
    assert!(
        matches!(selected.actions.as_slice(), [Action::OpenFile { source: selected_source, path }] if selected_source == &source && path == "/project/src/other.rs")
    );
}

#[test]
fn 열린_메뉴는_편집_세대_커서_체인_탭과_프로젝트_만료에서_닫힌다() {
    let (_, _, source) = document();
    let symbols = symbols();
    let rows = rows();
    let mut h = Harness::new();
    let mut views = Views::default();
    let mut current = source.clone();
    for change in 0..3 {
        let bar = views.entry(egui::ViewportId::ROOT, &current);
        let (initial, _) = h.frame(bar, scope(&current, &symbols, &rows, 22), Vec::new());
        h.ctx
            .memory_mut(|memory| memory.request_focus(initial.segments[3].id));
        h.frame(bar, scope(&current, &symbols, &rows, 22), Vec::new());
        h.frame(
            bar,
            scope(&current, &symbols, &rows, 22),
            vec![key(egui::Key::Enter)],
        );
        assert!(Popup::is_any_open(&h.ctx));
        if change == 0 {
            current.revision += 1;
        }
        if change == 1 {
            current.generation += 1;
        }
        h.frame(
            bar,
            scope(
                &current,
                &symbols,
                &rows,
                if change == 2 { 110 } else { 22 },
            ),
            Vec::new(),
        );
        assert!(!Popup::is_any_open(&h.ctx));
    }
    let live = HashMap::from([(source.project.clone(), layout(&source))]);
    let tree = ShellSlotTree::Leaf {
        slot_id: taide_model::ids::ShellSlotId::new(),
        project_id: source.project.clone(),
    };
    views.reconcile(
        &h.ctx,
        &live,
        Some(&tree),
        &taide_native_ui::shell::WindowScope::Main,
    );
    assert_eq!(views.entries.len(), 1);
    views.reconcile(
        &h.ctx,
        &live,
        None,
        &taide_native_ui::shell::WindowScope::Main,
    );
    assert!(views.entries.is_empty());
    views.reconcile(
        &h.ctx,
        &HashMap::new(),
        None,
        &taide_native_ui::shell::WindowScope::Main,
    );
    assert!(views.entries.is_empty());
    let mut bar = Bar::default();
    let (empty, paint) = h.frame(
        &mut bar,
        Scope {
            source: None,
            root: ROOT,
            caret: None,
            symbols: SymbolIndex::default(),
            rows: &rows,
        },
        Vec::new(),
    );
    assert!(empty.segments.is_empty());
    assert!(paint.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.job.text == "No active file")));
}

#[test]
fn breadcrumb_5000개_형제_메뉴는_표시영역만_그리고_end_선택과_세대_교체를_보존한다() {
    const SYMBOL_COUNT: usize = 5000;
    const MAX_PAINTED_ROWS: usize = 50;
    let (_, _, source) = document();
    let symbols = (0..SYMBOL_COUNT)
        .map(|index| Symbol {
            name: format!("Name{index}"),
            detail: String::new(),
            kind: SymbolKind::FUNCTION,
            tags: Vec::new(),
            parent: None,
            container_label: String::new(),
            selection: 0..1,
            bytes: 0..1,
        })
        .collect::<Vec<_>>();
    let mut h = Harness::new();
    let mut bar = Bar::default();
    let (initial, _) = h.frame(&mut bar, scope(&source, &symbols, &[], 0), Vec::new());
    assert!(initial.entries.is_empty());
    h.ctx
        .memory_mut(|memory| memory.request_focus(initial.segments[2].id));
    h.frame(&mut bar, scope(&source, &symbols, &[], 0), Vec::new());
    h.frame(
        &mut bar,
        scope(&source, &symbols, &[], 0),
        vec![key(egui::Key::Enter)],
    );
    let (visible, _) = h.frame(&mut bar, scope(&source, &symbols, &[], 0), Vec::new());
    assert!(visible.entries.len() < MAX_PAINTED_ROWS);
    let (last, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &[], 0),
        vec![key(egui::Key::End)],
    );
    assert!(last.entries.len() < MAX_PAINTED_ROWS);
    let (selected, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &[], 0),
        vec![key(egui::Key::Enter)],
    );
    assert!(
        matches!(selected.actions.as_slice(), [Action::RevealSymbol { index, .. }] if *index == SYMBOL_COUNT - 1)
    );
    assert!(!Popup::is_any_open(&h.ctx));
}

#[test]
fn breadcrumb_메뉴는_이름_키검색과_타임아웃_shift_tab_escape_포커스_복원을_보존한다() {
    let (_, _, source) = document();
    let symbols = symbols();
    let rows = rows();
    let mut h = Harness::new();
    let mut bar = Bar::default();
    let (initial, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    let trigger = initial.segments[1].id;
    h.ctx.memory_mut(|memory| memory.request_focus(trigger));
    h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::Enter)],
    );
    let (visible, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    let (typed, _) = h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![egui::Event::Text("c".into())],
    );
    assert!(typed.actions.is_empty());
    assert_eq!(
        h.ctx.memory(|memory| memory.focused()),
        Some(visible.entries[1].id)
    );
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::ArrowDown)],
    );
    assert_eq!(
        h.ctx.memory(|memory| memory.focused()),
        Some(visible.entries[1].id)
    );
    h.time += TYPEAHEAD_DELAY;
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![egui::Event::Text("o".into())],
    );
    assert_eq!(
        h.ctx.memory(|memory| memory.focused()),
        Some(visible.entries[0].id)
    );
    let shift_tab = egui::Event::Key {
        key: egui::Key::Tab,
        physical_key: Some(egui::Key::Tab),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::SHIFT,
    };
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![shift_tab],
    );
    assert_eq!(
        h.ctx.memory(|memory| memory.focused()),
        Some(visible.entries[0].id)
    );
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::Escape)],
    );
    assert!(!Popup::is_any_open(&h.ctx));
    assert_eq!(h.ctx.memory(|memory| memory.focused()), Some(trigger));
}

#[test]
fn breadcrumb_접근성_선택은_실제_현재_메뉴_파일에만_전달된다() {
    let (_, _, source) = document();
    let symbols = symbols();
    let rows = rows();
    let mut h = Harness::new();
    let mut bar = Bar::default();
    let (initial, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    h.ctx
        .memory_mut(|memory| memory.request_focus(initial.segments[1].id));
    h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    h.frame(
        &mut bar,
        scope(&source, &symbols, &rows, 22),
        vec![key(egui::Key::Enter)],
    );
    let (visible, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), Vec::new());
    let action = egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Click,
        target_node: visible.entries[0].id.accesskit_id(),
        target_tree: egui::accesskit::TreeId::ROOT,
        data: None,
    });
    let (selected, _) = h.frame(&mut bar, scope(&source, &symbols, &rows, 22), vec![action]);
    assert!(
        matches!(selected.actions.as_slice(), [Action::OpenFile { source: selected_source, path }] if selected_source == &source && path == "/project/src/other.rs")
    );
}
