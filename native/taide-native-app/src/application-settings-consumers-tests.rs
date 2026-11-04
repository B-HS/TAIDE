use std::collections::HashSet;

use taide_infra::perf::PerfRegistry;
use taide_model::font::FontFamily;
use taide_model::settings::TerminalCursorStyle;
use taide_native_ui::command_registry::{ActiveEditor, CommandContext, WindowKind, registry};
use taide_native_ui::intents::terminal_agent::Intent as TerminalIntent;
use taide_native_ui::keymap::catalog::{Overrides, rows};
use taide_native_ui::settings_code_controls::{
    Change as CodeChange, Selection, Switch as CodeSwitch,
};
use taide_native_ui::settings_controls::{Change, Numeric, Section, Switch};

use super::settings_sections_tests::{Scene, missing_cli_agents, nodes, texts_of, update};
use super::*;
use crate::settings_ports::Ports;

const TOC_SCREEN: egui::Vec2 = egui::vec2(1400.0, 900.0);
const ANCHOR_TOLERANCE: f32 = 1.0;
const ORIGINAL_TOC: [Section; 14] = [
    Section::Appearance,
    Section::Language,
    Section::Interface,
    Section::Notifications,
    Section::Editor,
    Section::Snippets,
    Section::Terminal,
    Section::Keymap,
    Section::Lsp,
    Section::Ai,
    Section::Plugins,
    Section::Sync,
    Section::Remote,
    Section::Performance,
];
const MONO_A: &str = "Fixture Mono A";
const MONO_B: &str = "Fixture Mono B";
const SANS: &str = "Fixture Sans";
const SEARCH: &str = "sans";
const CAT: &str = "/bin/cat";
const OTHER_CAT: &str = "/bin/../bin/cat";
const TERMINAL_FONT_SIZE: u32 = 19;

fn toc_rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    texts_of(output, label)
        .into_iter()
        .min_by(|left, right| left.left().total_cmp(&right.left()))
}

fn card_title(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    texts_of(output, label)
        .into_iter()
        .max_by(|left, right| left.left().total_cmp(&right.left()))
}

fn toc_button(output: &egui::FullOutput, label: &str) -> egui::Rect {
    nodes(output, label)
        .into_iter()
        .min_by(|left, right| left.left().total_cmp(&right.left()))
        .unwrap()
}

const DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
const POLL: std::time::Duration = std::time::Duration::from_millis(5);

/// Steps the app until `ready`, failing with only `phase` and the counts of `watched` texts.
fn wait_texts(
    scene: &mut Scene,
    phase: &str,
    watched: &[&str],
    ready: impl Fn(&NativeApplication, &egui::FullOutput) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = scene.step(Vec::new());
        if ready(scene.application(), &output) {
            return output;
        }
        assert!(
            Instant::now() < deadline,
            "phase={phase}, counts={:?}, nodes={:?}",
            watched
                .iter()
                .map(|text| (*text, texts_of(&output, text)))
                .collect::<Vec<_>>(),
            watched
                .iter()
                .map(|text| (*text, nodes(&output, text).len()))
                .collect::<Vec<_>>()
        );
        std::thread::sleep(POLL);
    }
}

#[test]
fn 실제_목차는_원본_14항목_순서이고_클릭한_카드를_목차_첫_항목과_같은_높이로_옮긴다() {
    let mut scene = Scene::launch(None, |_| {});
    let registry: &'static PerfRegistry = Box::leak(Box::new(PerfRegistry::new()));
    registry.set_enabled(true);
    scene.ports(Ports {
        perf: registry,
        agents: missing_cli_agents(),
        ..Default::default()
    });
    let loading = scene.text("settings.loading");
    let performance = scene.text(Section::Performance.title());
    wait_texts(&mut scene, "sections loaded", &[], |_, output| {
        texts_of(output, &loading).is_empty() && !texts_of(output, &performance).is_empty()
    });
    scene.resize(TOC_SCREEN);
    let labels = ORIGINAL_TOC
        .iter()
        .map(|section| scene.text(section.title()))
        .collect::<Vec<_>>();
    let output = wait_texts(&mut scene, "toc", &[], |_, output| {
        labels.iter().all(|label| toc_rect(output, label).is_some())
    });
    let tops = labels
        .iter()
        .map(|label| toc_rect(&output, label).unwrap().top())
        .collect::<Vec<_>>();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    let appearance = &labels[0];
    assert!(
        card_title(&output, appearance).unwrap().left()
            > toc_rect(&output, appearance).unwrap().right()
    );
    let inset =
        card_title(&output, appearance).unwrap().top() - toc_button(&output, appearance).top();
    let lsp = scene.text(Section::Lsp.title());
    scene.click(toc_rect(&output, &lsp).unwrap().center());
    let active = |scene: &mut Scene| {
        scene
            .application()
            .settings_state
            .settings_views
            .inspection()
            .values()
            .map(|view| view.active)
            .collect::<Vec<_>>()
    };
    assert_eq!(active(&mut scene), vec![Section::Lsp]);
    let watched = [lsp.as_str(), appearance.as_str()];
    let output = wait_texts(&mut scene, "lsp anchored", &watched, |_, output| {
        card_title(output, &lsp).is_some_and(|title| {
            (title.top() - toc_button(output, appearance).top() - inset).abs() <= ANCHOR_TOLERANCE
        })
    });
    assert!(toc_button(&output, appearance).top() >= 0.0);
    assert!(card_title(&output, &lsp).unwrap().left() > toc_rect(&output, &lsp).unwrap().right());
    scene.click(toc_rect(&output, &performance).unwrap().center());
    wait_texts(&mut scene, "performance visible", &[], |_, output| {
        card_title(output, &performance)
            .is_some_and(|title| title.top() >= 0.0 && title.bottom() <= TOC_SCREEN.y)
    });
}

#[test]
fn 실제_글꼴_picker는_모노스페이스_필터_검색_시스템기본과_선택을_설정에_반영한다() {
    let mut scene = Scene::launch(None, |_| {});
    scene.ports(Ports {
        agents: missing_cli_agents(),
        font_families: std::sync::Arc::new(|| {
            vec![
                FontFamily {
                    name: MONO_A.into(),
                    monospaced: true,
                },
                FontFamily {
                    name: MONO_B.into(),
                    monospaced: true,
                },
                FontFamily {
                    name: SANS.into(),
                    monospaced: false,
                },
            ]
        }),
        ..Default::default()
    });
    let system = scene.text("settings.fontFamilySystemDefault");
    let filter = scene.text("settings.fontFamilyMonospaceOnly");
    let watched = [system.as_str(), filter.as_str(), MONO_A, MONO_B, SANS];
    let output = wait_texts(&mut scene, "pickers", &watched, |_, output| {
        texts_of(output, &system).len() == 2 && nodes(output, &filter).len() >= 2
    });
    let trigger = texts_of(&output, &system)[0];
    scene.click(trigger.center());
    let output = wait_texts(&mut scene, "monospace list", &watched, |_, output| {
        nodes(output, MONO_A).len() == 1
    });
    assert_eq!(nodes(&output, MONO_B).len(), 1);
    assert!(
        nodes(&output, SANS).is_empty(),
        "monospace filter is on by default"
    );
    assert_eq!(texts_of(&output, &system).len(), 3);
    scene.keys(&[egui::Key::Escape]);
    let output = wait_texts(&mut scene, "closed", &watched, |_, output| {
        nodes(output, MONO_A).is_empty()
    });
    scene.click(nodes(&output, &filter)[0].center());
    scene.click(trigger.center());
    let output = wait_texts(&mut scene, "all fonts", &watched, |_, output| {
        nodes(output, SANS).len() == 1
    });
    assert_eq!(nodes(&output, MONO_A).len(), 1);
    scene.step(vec![egui::Event::Text(SEARCH.into())]);
    let output = wait_texts(&mut scene, "search", &watched, |_, output| {
        nodes(output, MONO_A).is_empty() && nodes(output, SANS).len() == 1
    });
    scene.click(nodes(&output, SANS)[0].center());
    let output = wait_texts(&mut scene, "saved", &watched, |application, output| {
        application
            .services
            .state
            .settings
            .read()
            .editor_font_family
            .as_deref()
            == Some(SANS)
            && nodes(output, SANS).is_empty()
            && texts_of(output, &system).len() == 1
    });
    assert!(nodes(&output, MONO_B).is_empty());
    let saved =
        std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap();
    assert!(saved.contains(SANS));
    assert_eq!(
        crate::editor_fonts::Families::new(&scene.application().services.state.settings.read())
            .editor,
        crate::terminal_fonts::Requested::Named(SANS.into())
    );
    scene.click(trigger.center());
    let output = wait_texts(&mut scene, "reopened", &watched, |_, output| {
        texts_of(output, &system).len() == 2
    });
    let default_entry = texts_of(&output, &system)
        .into_iter()
        .filter(|rect| rect.top() > trigger.top())
        .min_by(|left, right| left.top().total_cmp(&right.top()))
        .unwrap();
    scene.click(default_entry.center());
    wait_texts(
        &mut scene,
        "system default",
        &watched,
        |application, output| {
            application
                .services
                .state
                .settings
                .read()
                .editor_font_family
                .as_deref()
                .is_none_or(str::is_empty)
                && texts_of(output, &system).len() == 2
                && nodes(output, SANS).is_empty()
        },
    );
    assert_eq!(
        crate::editor_fonts::Families::new(&scene.application().services.state.settings.read())
            .editor,
        crate::terminal_fonts::Requested::Default
    );
}

#[test]
fn 실제_빈_편집기_환영화면은_설정을_끄면_사라지고_빈_편집기_문구를_보인다() {
    let mut scene = Scene::launch_empty_pane(|_| {});
    let hint = scene.text("app.openFolderHint");
    let empty = scene.text("editor.noFileOpen");
    let output = scene.until("welcome", |_, output| !texts_of(output, &hint).is_empty());
    assert!(texts_of(&output, &empty).is_empty());
    update(
        &mut scene,
        Change::Switch(Switch::WelcomeOnEmptyEditor, false),
    );
    let output = scene.until("no welcome", |_, output| {
        !texts_of(output, &empty).is_empty()
    });
    assert!(texts_of(&output, &hint).is_empty());
    assert!(
        !scene
            .application()
            .services
            .state
            .settings
            .read()
            .welcome_on_empty_editor
    );
    update(
        &mut scene,
        Change::Switch(Switch::WelcomeOnEmptyEditor, true),
    );
    scene.until("welcome again", |_, output| {
        !texts_of(output, &hint).is_empty() && texts_of(output, &empty).is_empty()
    });
}

fn focused_pane(scene: &mut Scene) -> (ProjectId, PaneId) {
    let state = &scene.application().services.state;
    let project = state.session.read().active_project.clone().unwrap();
    let pane = state.layouts.read()[&project].focused_pane.clone();
    (project, pane)
}

fn cursor_of(application: &NativeApplication, session: &str) -> taide_native_terminal::CursorStyle {
    application
        .terminal_agent_state
        .terminals
        .hub()
        .get(session)
        .unwrap()
        .snapshot(|snapshot| snapshot.core.cursor_style())
        .unwrap()
        .unwrap()
}

#[test]
fn 실제_터미널_설정은_실행중_세션의_커서_깜빡임_글자크기에_반영되고_shell은_새_세션부터_쓴다() {
    let mut scene = Scene::launch_empty_pane(|state| {
        state.settings.write().shell_override = Some(CAT.into());
    });
    let (project, pane) = focused_pane(&mut scene);
    scene
        .application()
        .apply_terminal_agent_intent(TerminalIntent::NewTerminal {
            project: project.clone(),
            pane: pane.clone(),
        });
    wait_texts(&mut scene, "first terminal", &[], |application, _| {
        application
            .services
            .terminal
            .sessions_for_project(&project)
            .first()
            .is_some_and(|session| {
                session.running && application.has_terminal_agent_session(&session.id)
            })
    });
    let first = scene
        .application()
        .services
        .terminal
        .sessions_for_project(&project)[0]
        .clone();
    assert_eq!(first.shell, CAT);
    let before = cursor_of(scene.application(), &first.id);
    let blink = scene
        .application()
        .services
        .state
        .settings
        .read()
        .terminal_cursor_blink;
    update(
        &mut scene,
        Change::Code(CodeChange::Selection(Selection::TerminalCursor(
            TerminalCursorStyle::Underline,
        ))),
    );
    update(
        &mut scene,
        Change::Code(CodeChange::Switch(CodeSwitch::TerminalCursorBlink, !blink)),
    );
    update(
        &mut scene,
        Change::Numeric(Numeric::TerminalFontSize, TERMINAL_FONT_SIZE),
    );
    wait_texts(
        &mut scene,
        "running terminal follows",
        &[],
        |application, _| {
            let settings = application.services.state.settings.read();
            settings.terminal_font_size == TERMINAL_FONT_SIZE
                && settings.terminal_cursor_blink != blink
                && cursor_of(application, &first.id) == crate::terminal_settings::cursor(&settings)
                && application
                    .terminal_agent_state
                    .terminal_appearance
                    .font
                    .size
                    == TERMINAL_FONT_SIZE as f32
        },
    );
    assert_ne!(cursor_of(scene.application(), &first.id), before);
    update(
        &mut scene,
        Change::Code(CodeChange::Shell(OTHER_CAT.into())),
    );
    wait_texts(&mut scene, "shell saved", &[], |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .shell_override
            .as_deref()
            == Some(OTHER_CAT)
    });
    let sessions = scene
        .application()
        .services
        .terminal
        .sessions_for_project(&project);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].shell, CAT, "a running terminal keeps its shell");
    scene
        .application()
        .apply_terminal_agent_intent(TerminalIntent::NewTerminal {
            project: project.clone(),
            pane,
        });
    wait_texts(&mut scene, "second terminal", &[], |application, _| {
        application
            .services
            .terminal
            .sessions_for_project(&project)
            .iter()
            .any(|session| session.id != first.id && session.shell == OTHER_CAT)
    });
}

#[test]
fn override로_라우팅되는_catalog_명령의_앱_실행_경로_유무를_문맥에서_판정한다() {
    let context = CommandContext {
        active_project: Some(ProjectId::new()),
        focused_shell_slot: Some(taide_model::ids::ShellSlotId::new()),
        active_editor_actions: Some(registry().unwrap().editor_action_ids(ActiveEditor {
            is_read_only: false,
            has_folding: true,
        })),
        window: WindowKind::Main,
    };
    let is_mac = cfg!(target_os = "macos");
    let catalog = rows(&Overrides::default(), is_mac).unwrap();
    let ids = catalog
        .iter()
        .map(|row| row.id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(ids.len(), catalog.len(), "catalog rows have unique ids");
    let accepted = catalog
        .iter()
        .filter(|row| crate::command_dispatch::accepts(&row.id, true, &context))
        .count();
    let rejected = catalog
        .iter()
        .filter(|row| !crate::command_dispatch::accepts(&row.id, true, &context))
        .map(|row| row.id.as_str())
        .collect::<Vec<_>>();
    assert!(accepted > 0);
    assert!(rejected.iter().all(|id| !crate::shell_keymap::supports(id)));
    println!(
        "settings-67 accepted={accepted} rejected={}: {rejected:?}",
        rejected.len()
    );
}
