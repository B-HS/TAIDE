use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;

use taide_model::app::{AppFileTarget, PromptTemplateId};
use taide_model::ids::{ShellSlotId, TabId};
use taide_model::layout::PaneNode;
use taide_model::project::ShellSlotTree;
use taide_model::system::AppDataPathKind;
use taide_native_ui::commands::ShellMutation;
use zip::write::SimpleFileOptions;

use super::settings_sections_tests::{Scene, errors, nodes, successes, texts_of};
use super::*;
use crate::application::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, pointer, project_ref, step,
};

const AUXILIARY_SCREEN: [f32; 2] = [1400.0, 9000.0];
const MAIN_TAB_BAR_BOTTOM: f32 =
    taide_native_ui::shell::TITLE_HEIGHT + taide_native_ui::shell::TAB_HEIGHT;
const AUXILIARY_SLOT: u32 = 1;
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
const POLL: std::time::Duration = std::time::Duration::from_millis(5);
const VSIX_PLUGIN_ID: &str = "acme-fixture-themes";
const FIXTURE_EXTENSION: &str = "Fixture Themes";
const NIGHT_ID: &str = "acme-fixture-themes-fixture-night";
const DAY_ID: &str = "acme-fixture-themes-fixture-day";

fn manifest(id: &str, name: &str) -> String {
    format!(
        r#"{{"manifestVersion":1,"id":"{id}","name":"{name}","version":"1.0.0","contributes":{{}}}}"#
    )
}

fn write_plugin(plugins: &Path, directory: &str, id: &str, name: &str) {
    let root = plugins.join(directory);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("taide-plugin.json"), manifest(id, name)).unwrap();
}

fn write_zip(path: &Path, entries: &[(&str, &str)]) {
    let file = std::fs::File::create(path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    for (name, content) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

fn write_vsix(path: &Path) {
    let package = r#"{
        "name": "fixture-themes",
        "displayName": "Fixture Themes",
        "publisher": "acme",
        "version": "1.0.0",
        "contributes": {
            "themes": [
                { "label": "Fixture Night", "uiTheme": "vs-dark", "path": "./themes/night.json" },
                { "label": "Fixture Day", "uiTheme": "vs", "path": "./themes/day.json" }
            ],
            "languages": [{ "id": "fixturelang", "extensions": [".fx"] }],
            "grammars": [{ "language": "fixturelang", "scopeName": "source.fx", "path": "./syntaxes/fx.tmLanguage.json" }]
        }
    }"#;
    let night = r##"{ "colors": { "editor.background": "#1e1e1e", "editor.foreground": "#d4d4d4", "foreground": "#d4d4d4" } }"##;
    let day = r##"{ "colors": { "editor.background": "#ffffff", "editor.foreground": "#1e1e1e", "foreground": "#1e1e1e" } }"##;
    let grammar = r#"{ "scopeName": "source.fx", "patterns": [] }"#;
    write_zip(
        path,
        &[
            ("extension/package.json", package),
            ("extension/themes/night.json", night),
            ("extension/themes/day.json", day),
            ("extension/syntaxes/fx.tmLanguage.json", grammar),
        ],
    );
}

struct Picks {
    answers: Mutex<Vec<Option<PathBuf>>>,
    requests: Mutex<Vec<crate::file_dialogs::OpenRequest>>,
}

fn install_picker(scene: &mut Scene) -> Arc<Picks> {
    let picks = Arc::new(Picks {
        answers: Mutex::new(Vec::new()),
        requests: Mutex::new(Vec::new()),
    });
    let recorded = picks.clone();
    scene.ports(crate::settings_ports::Ports {
        pick_file: Arc::new(move |_, request| {
            recorded.requests.lock().unwrap().push(request);
            recorded.answers.lock().unwrap().pop().flatten()
        }),
        ..Default::default()
    });
    picks
}

fn answer(picks: &Picks, path: Option<PathBuf>) {
    picks.answers.lock().unwrap().push(path);
}

fn lowest(output: &egui::FullOutput, text: &str) -> egui::Rect {
    *texts_of(output, text)
        .last()
        .unwrap_or_else(|| panic!("painted: {text}"))
}

fn brief(
    scene: &mut Scene,
    phase: &str,
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
            "phase={phase}, errors={:?}, successes={:?}",
            errors(scene.application()),
            successes(scene.application())
        );
        std::thread::sleep(POLL);
    }
}

fn themes(application: &NativeApplication) -> Vec<taide_model::theme::ThemeSummary> {
    taide_runtime::theme_actions::theme_list(&application.services.state).unwrap()
}

#[test]
fn 실제_plugins_section은_목록_zip_설치_새로고침_제거와_폴더_경로를_원본대로_처리한다() {
    let mut scene = Scene::launch(None, |state| {
        let plugins = state.paths.plugins_dir();
        write_plugin(&plugins, "good", "good", "Good Plugin");
        write_plugin(&plugins, "bad", "other-id", "Bad Plugin");
    });
    let picks = install_picker(&mut scene);
    let plugins_dir = scene.application().services.state.paths.plugins_dir();
    let inputs = plugins_dir.parent().unwrap().join("inputs");
    std::fs::create_dir_all(&inputs).unwrap();
    let mismatch = scene.text("settings.pluginError.idMismatch");
    scene.until("plugin rows", |_, output| {
        !texts_of(output, "Good Plugin").is_empty() && !texts_of(output, &mismatch).is_empty()
    });

    let install = scene.text("settings.pluginInstallButton");
    let archive = inputs.join("zipped.zip");
    write_zip(
        &archive,
        &[("taide-plugin.json", &manifest("zipped", "Zipped Plugin"))],
    );
    answer(&picks, Some(archive));
    let output = scene.step(Vec::new());
    scene.click(texts_of(&output, &install)[0].center());
    let installed = scene.text("settings.pluginInstallSuccess");
    scene.until("zip installed", |application, output| {
        successes(application).contains(&installed) && !texts_of(output, "Zipped Plugin").is_empty()
    });
    assert!(plugins_dir.join("zipped/taide-plugin.json").is_file());
    {
        let requests = picks.requests.lock().unwrap();
        assert_eq!(requests[0].filter_name, "Plugin Archive");
        assert_eq!(requests[0].extensions, ["zip"]);
        assert_eq!(
            requests[0].title,
            presentation::message(
                &scene_locale(&mut scene),
                "settings.pluginInstallDialogTitle",
                &[]
            )
        );
    }

    let toasts = scene.application().toasts.inspection().len();
    answer(&picks, None);
    let output = scene.step(Vec::new());
    scene.click(texts_of(&output, &install)[0].center());
    for _ in 0..4 {
        scene.step(Vec::new());
    }
    assert_eq!(picks.requests.lock().unwrap().len(), 2);
    assert_eq!(scene.application().toasts.inspection().len(), toasts);

    let broken = inputs.join("broken.zip");
    std::fs::write(&broken, "not a zip archive").unwrap();
    answer(&picks, Some(broken));
    let failures = errors(scene.application()).len();
    let output = scene.step(Vec::new());
    scene.click(texts_of(&output, &install)[0].center());
    scene.until("broken zip rejected", |application, _| {
        errors(application).len() > failures
    });

    write_plugin(&plugins_dir, "later", "later", "Later Plugin");
    let reload = scene.text("settings.pluginsReload");
    let output = scene.step(Vec::new());
    scene.click(texts_of(&output, &reload)[0].center());
    scene.until("reloaded", |_, output| {
        !texts_of(output, "Later Plugin").is_empty()
    });

    let uninstall = scene.text("settings.pluginUninstallButton");
    let output = scene.step(Vec::new());
    let row = texts_of(&output, "Zipped Plugin")[0];
    let trash = nodes(&output, &uninstall)
        .into_iter()
        .min_by(|left, right| {
            (left.center().y - row.center().y)
                .abs()
                .total_cmp(&(right.center().y - row.center().y).abs())
        })
        .unwrap();
    scene.click(trash.center());
    let title = presentation::message(
        &scene_locale(&mut scene),
        "settings.pluginUninstallConfirmTitle",
        &[("name", "Zipped Plugin")],
    );
    let output = scene.until("uninstall dialog", |_, output| {
        !texts_of(output, &title).is_empty()
    });
    scene.click(lowest(&output, &uninstall).center());
    let removed = scene.text("settings.pluginUninstallSuccess");
    scene.until("uninstalled", |application, output| {
        successes(application).contains(&removed) && texts_of(output, "Zipped Plugin").is_empty()
    });
    assert!(!plugins_dir.join("zipped").exists());

    std::fs::remove_dir_all(plugins_dir.join("later")).unwrap();
    let output = scene.step(Vec::new());
    let row = texts_of(&output, "Later Plugin")[0];
    let trash = nodes(&output, &uninstall)
        .into_iter()
        .min_by(|left, right| {
            (left.center().y - row.center().y)
                .abs()
                .total_cmp(&(right.center().y - row.center().y).abs())
        })
        .unwrap();
    scene.click(trash.center());
    let failures = errors(scene.application()).len();
    let output = scene.until("second dialog", |_, output| {
        texts_of(output, &uninstall).len() == 1
    });
    scene.click(lowest(&output, &uninstall).center());
    scene.until("missing plugin uninstall fails", |application, _| {
        errors(application).len() > failures
    });

    let recorded = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
    struct Recording(Arc<Mutex<Vec<PathBuf>>>);
    impl taide_runtime::PlatformServices for Recording {
        fn open_path(&self, _: &Path) -> AppResult<()> {
            Ok(())
        }
        fn reveal_item_in_dir(&self, path: &Path) -> AppResult<()> {
            self.0.lock().unwrap().push(path.into());
            Ok(())
        }
        fn open_url(&self, _: &str) -> AppResult<()> {
            Ok(())
        }
        fn send_notification(&self, _: &str, _: &str) -> AppResult<()> {
            Ok(())
        }
    }
    taide_runtime::system_actions::system_open_app_data_path(
        &scene.application().services.state,
        &Recording(recorded.clone()),
        AppDataPathKind::Plugins,
    )
    .unwrap();
    assert_eq!(*recorded.lock().unwrap(), vec![plugins_dir.clone()]);
}

#[test]
fn 실제_sections_요청_제출이_실패하면_오류_알림은_한_번만_뜬다() {
    let mut scene = Scene::launch(None, |_| {});
    let reload = scene.text("settings.pluginsReload");
    let empty = scene.text("settings.pluginsEmpty");
    let output = brief(&mut scene, "plugins listed", |_, output| {
        !nodes(output, &reload).is_empty() && !texts_of(output, &empty).is_empty()
    });
    let before = errors(scene.application());
    let bridge = scene.application().bridge.take();
    scene.click(nodes(&output, &reload)[0].center());
    let after = errors(scene.application());
    scene.application().bridge = bridge;
    assert_eq!(after.len(), before.len() + 1, "{after:?}");
    assert!(
        !scene
            .application()
            .settings_state
            .settings_views
            .sections_data_mut()
            .is_reloading_plugins()
    );
}

fn scene_locale(scene: &mut Scene) -> taide_model::locale::ResolvedLocale {
    scene.application().locale.clone()
}

#[test]
fn 실제_vsix_가져오기는_테마_저장_중복_복사본과_문법_3상태를_원본대로_처리한다() {
    let mut scene = Scene::launch(None, |_| {});
    let picks = install_picker(&mut scene);
    let plugins_dir = scene.application().services.state.paths.plugins_dir();
    let inputs = plugins_dir.parent().unwrap().join("inputs");
    std::fs::create_dir_all(&inputs).unwrap();
    let vsix = inputs.join("fixture.vsix");
    write_vsix(&vsix);
    let import = scene.text("settings.pluginImportVsixButton");
    let themes_heading = scene.text("settings.pluginImportVsixThemesSection");
    let open_dialog = |scene: &mut Scene| {
        answer(&picks, Some(vsix.clone()));
        let output = scene.until("import button", |_, output| {
            texts_of(output, &import).len() == 1
        });
        let picked = picks.requests.lock().unwrap().len();
        scene.click(texts_of(&output, &import)[0].center());
        assert_eq!(picks.requests.lock().unwrap().len(), picked + 1);
        brief(scene, "vsix extracted", |application, _| {
            !application
                .settings_state
                .settings_views
                .sections_data()
                .is_extracting_vsix()
        });
        brief(scene, "vsix dialog", |_, output| {
            !texts_of(output, &themes_heading).is_empty()
                && !texts_of(output, FIXTURE_EXTENSION).is_empty()
        });
        scene.step(Vec::new())
    };
    let before = themes(scene.application()).len();
    let output = open_dialog(&mut scene);
    assert_eq!(picks.requests.lock().unwrap()[0].filter_name, "VSIX");
    assert_eq!(picks.requests.lock().unwrap()[0].extensions, ["vsix"]);
    let save = scene.text("common.save");
    scene.click(lowest(&output, &save).center());
    let saved = presentation::message(
        &scene_locale(&mut scene),
        "settings.themeImportSuccess",
        &[("count", "2"), ("extension", FIXTURE_EXTENSION)],
    );
    brief(&mut scene, "themes saved", |application, _| {
        successes(application).contains(&saved)
    });
    let ids = themes(scene.application())
        .into_iter()
        .map(|theme| theme.id)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), before + 2);
    assert!(ids.contains(&NIGHT_ID.to_owned()) && ids.contains(&DAY_ID.to_owned()));

    let grammars = brief(&mut scene, "grammar button", |_, output| {
        texts_of(output, &import).len() == 3
    });
    scene.click(texts_of(&grammars, &import)[1].center());
    let imported = scene.text("settings.pluginImportVsixSuccess");
    brief(&mut scene, "grammars imported", |application, output| {
        successes(application).contains(&imported) && texts_of(output, &import).len() == 2
    });
    assert!(
        plugins_dir
            .join(VSIX_PLUGIN_ID)
            .join("taide-plugin.json")
            .is_file()
    );

    scene.step(vec![egui::Event::PointerMoved(egui::pos2(2.0, 2.0))]);
    for _ in 0..4 {
        scene.step(Vec::new());
    }
    scene.step(vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: Some(egui::Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    let output = scene.step(Vec::new());
    assert!(texts_of(&output, &themes_heading).is_empty());
    assert!(scene.context().dismissal_layers().is_empty());
    let output = open_dialog(&mut scene);
    let duplicate = presentation::message(
        &scene_locale(&mut scene),
        "settings.themeImportDuplicate",
        &[("name", NIGHT_ID)],
    );
    assert_eq!(texts_of(&output, &duplicate).len(), 1);
    scene.click(lowest(&output, &save).center());
    let confirm = scene.text("common.confirm");
    brief(&mut scene, "overwrite confirm", |_, output| {
        texts_of(output, &confirm).len() == 2
    });
    let output = scene.step(Vec::new());
    scene.click(lowest(&output, &confirm).center());
    brief(&mut scene, "copies saved", |application, _| {
        themes(application).len() == before + 4
    });
    let copies = themes(scene.application())
        .into_iter()
        .filter(|theme| ![NIGHT_ID, DAY_ID].contains(&theme.id.as_str()))
        .filter(|theme| theme.name.starts_with("Fixture"))
        .collect::<Vec<_>>();
    let copy_name = presentation::message(
        &scene_locale(&mut scene),
        "themeEditor.duplicateNameTemplate",
        &[("name", "Fixture Night")],
    );
    assert!(
        copies.iter().any(|theme| theme.name == copy_name),
        "{copies:?}"
    );

    let output = brief(&mut scene, "grammar button again", |_, output| {
        texts_of(output, &import).len() == 3
    });
    let failures = errors(scene.application()).len();
    scene.click(texts_of(&output, &import)[1].center());
    let already = presentation::message(
        &scene_locale(&mut scene),
        "error.plugin.alreadyInstalled",
        &[("pluginId", VSIX_PLUGIN_ID)],
    );
    brief(
        &mut scene,
        "grammar import failure is inline",
        |_, output| !texts_of(output, &already).is_empty(),
    );
    assert_eq!(errors(scene.application()).len(), failures);
}

#[derive(Default)]
struct Viewports {
    events: HashMap<egui::ViewportId, Vec<egui::Event>>,
    close: HashSet<egui::ViewportId>,
    outputs: HashMap<egui::ViewportId, egui::FullOutput>,
}

fn install_viewports(context: &egui::Context) -> Rc<RefCell<Viewports>> {
    let viewports = Rc::new(RefCell::new(Viewports::default()));
    let shared = viewports.clone();
    context.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
        let id = viewport.ids.this;
        let (events, close) = {
            let mut state = shared.borrow_mut();
            (
                state.events.remove(&id).unwrap_or_default(),
                state.close.remove(&id),
            )
        };
        let info = egui::ViewportInfo {
            events: if close {
                vec![egui::ViewportEvent::Close]
            } else {
                Vec::new()
            },
            ..Default::default()
        };
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: id,
                viewports: std::iter::once((id, info)).collect(),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(AUXILIARY_SCREEN[0], AUXILIARY_SCREEN[1]),
                )),
                events,
                ..Default::default()
            },
            |ui| (viewport.viewport_ui_cb)(ui),
        );
        output.textures_delta.clear();
        shared.borrow_mut().outputs.insert(id, output);
    });
    viewports
}

fn settings_tab() -> Tab {
    Tab {
        id: TabId::new(),
        kind: TabKind::Settings,
        title: "Settings".into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

fn main_tab_bar() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(f32::INFINITY, MAIN_TAB_BAR_BOTTOM),
    )
}

fn pane_holding(node: &PaneNode, tab: &TabId) -> Option<PaneId> {
    match node {
        PaneNode::Leaf { id, tabs, .. } => tabs
            .iter()
            .any(|candidate| &candidate.id == tab)
            .then(|| id.clone()),
        PaneNode::Split { children, .. } => {
            children.iter().find_map(|child| pane_holding(child, tab))
        }
    }
}

fn main_target(
    application: &mut NativeApplication,
    context: &egui::Context,
    label: &str,
    within: egui::Rect,
) -> egui::Rect {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = step(application, context, Vec::new());
        let painted = texts_of(&output, label)
            .into_iter()
            .find(|rect| within.contains_rect(*rect));
        if let Some(rect) = node_rect(&output, label, within).or(painted) {
            return rect;
        }
        assert!(Instant::now() < deadline, "main window target {label}");
        std::thread::sleep(POLL);
    }
}

fn pane_tabs(node: &PaneNode, pane: &PaneId) -> Vec<Tab> {
    match node {
        PaneNode::Leaf { id, tabs, .. } if id == pane => tabs.clone(),
        PaneNode::Leaf { .. } => Vec::new(),
        PaneNode::Split { children, .. } => children
            .iter()
            .flat_map(|child| pane_tabs(child, pane))
            .collect(),
    }
}

#[test]
fn 실제_보조_창_settings는_자기_프로젝트_창_pane에_settings_json과_프롬프트를_열고_닫히면_메인에서_새로_mount된다()
 {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-settings-owner-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let alpha_root = fixture.directory.join("alpha");
    let beta_root = fixture.directory.join("beta");
    for directory in [&data, &alpha_root, &beta_root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let alpha = ProjectId::new();
    let beta = ProjectId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &alpha_root, "alpha", false);
    insert_project(&state, &beta, &beta_root, "beta", false);
    let settings = settings_tab();
    {
        let mut layouts = state.layouts.write();
        let PaneNode::Leaf { tabs, active, .. } = &mut layouts.get_mut(&alpha).unwrap().root else {
            panic!("default layout is one pane");
        };
        *tabs = vec![settings.clone()];
        *active = Some(settings.id.clone());
    }
    {
        let slot = ShellSlotId::new();
        let mut session = state.session.write();
        session.projects = vec![
            project_ref(&alpha, &alpha_root, "alpha"),
            project_ref(&beta, &beta_root, "beta"),
        ];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let move_to_new_window = presentation::message(&application.locale, "tab.moveToNewWindow", &[]);
    let settings_label = main_target(application, &context, &settings.title, main_tab_bar());
    step(
        application,
        &context,
        pointer(settings_label.center(), egui::PointerButton::Secondary),
    );
    let menu_item = main_target(
        application,
        &context,
        &move_to_new_window,
        egui::Rect::EVERYTHING,
    );
    step(
        application,
        &context,
        pointer(menu_item.center(), egui::PointerButton::Primary),
    );
    let deadline = Instant::now() + DEADLINE;
    let (viewport, auxiliary_pane) = loop {
        step(application, &context, Vec::new());
        let moved = services.state.layouts.read()[&alpha]
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == AUXILIARY_SLOT)
            .and_then(|window| pane_holding(&window.root, &settings.id));
        let window = application
            .shell_state
            .auxiliary_windows
            .find(&alpha, AUXILIARY_SLOT)
            .map(|window| window.viewport);
        if let (Some(viewport), Some(pane)) = (window, moved) {
            break (viewport, pane);
        }
        assert!(
            Instant::now() < deadline,
            "settings tab moved to a new window"
        );
        std::thread::sleep(POLL);
    };
    application
        .controller
        .submit(ShellMutation::ActivateProject(beta.clone()))
        .unwrap();
    let deadline = Instant::now() + DEADLINE;
    while application.controller.snapshot().focused_project() != Some(&beta) {
        step(application, &context, Vec::new());
        assert!(Instant::now() < deadline, "beta active in the main window");
        std::thread::sleep(POLL);
    }
    let beta_tabs = crate::tabs::tabs_in(&services.state.layouts.read()[&beta].root).len();
    let settings_file = presentation::message(&application.locale, "app.openSettingsFile", &[]);
    let edit = presentation::message(&application.locale, "prompts.editEntry", &[]);
    let prompt_title = presentation::message(&application.locale, "prompts.autoTabTitle", &[]);
    let wait = |application: &mut NativeApplication,
                phase: &str,
                ready: &dyn Fn(&NativeApplication, &Viewports) -> bool| {
        let deadline = Instant::now() + DEADLINE;
        loop {
            step(application, &context, Vec::new());
            if ready(application, &viewports.borrow()) {
                return;
            }
            assert!(Instant::now() < deadline, "phase={phase}");
            std::thread::sleep(POLL);
        }
    };
    let click_in = |application: &mut NativeApplication, label: &str, pick: usize| {
        let rect = {
            let viewports = viewports.borrow();
            let output = &viewports.outputs[&viewport];
            let mut matches = nodes(output, label);
            if matches.is_empty() {
                matches = texts_of(output, label);
            }
            matches[pick]
        };
        viewports.borrow_mut().events.insert(
            viewport,
            crate::application::project_rail_tests::pointer(
                rect.center(),
                egui::PointerButton::Primary,
            ),
        );
        step(application, &context, Vec::new());
    };
    wait(
        application,
        "settings drawn in the auxiliary viewport",
        &|_, viewports| {
            viewports.outputs.get(&viewport).is_some_and(|output| {
                !texts_of(output, &settings_file).is_empty() && !nodes(output, &edit).is_empty()
            })
        },
    );
    let has_tab = |application: &NativeApplication, kind: &TabKind| {
        let layouts = application.services.state.layouts.read();
        layouts[&alpha]
            .auxiliary_windows
            .first()
            .is_some_and(|window| {
                pane_tabs(&window.root, &auxiliary_pane)
                    .iter()
                    .any(|tab| tab.kind == *kind)
            })
    };
    let prompt = TabKind::AppFile {
        target: AppFileTarget::Prompt {
            id: PromptTemplateId::AutoTabDefault,
        },
    };
    click_in(application, &edit, 0);
    wait(
        application,
        "prompt in the auxiliary pane",
        &|application, _| has_tab(application, &prompt),
    );
    {
        let layouts = services.state.layouts.read();
        let tabs = pane_tabs(&layouts[&alpha].auxiliary_windows[0].root, &auxiliary_pane);
        assert_eq!(
            tabs.iter()
                .find(|tab| tab.kind == prompt)
                .map(|tab| tab.title.as_str()),
            Some(prompt_title.as_str())
        );
    }
    wait(application, "settings tab label", &|_, viewports| {
        viewports
            .outputs
            .get(&viewport)
            .is_some_and(|output| !texts_of(output, &settings.title).is_empty())
    });
    click_in(application, &settings.title, 0);
    wait(application, "settings shown again", &|_, viewports| {
        viewports
            .outputs
            .get(&viewport)
            .is_some_and(|output| !texts_of(output, &settings_file).is_empty())
    });
    click_in(application, &settings_file, 0);
    let settings_file_tab = TabKind::AppFile {
        target: AppFileTarget::Settings,
    };
    wait(
        application,
        "settings.json in the auxiliary pane",
        &|application, _| has_tab(application, &settings_file_tab),
    );
    assert_eq!(
        crate::tabs::tabs_in(&services.state.layouts.read()[&beta].root).len(),
        beta_tabs
    );
    click_in(application, &settings.title, 0);
    wait(
        application,
        "settings shown before closing",
        &|_, viewports| {
            viewports
                .outputs
                .get(&viewport)
                .is_some_and(|output| !texts_of(output, &settings_file).is_empty())
        },
    );
    let plugins_title = presentation::message(&application.locale, "settings.plugins", &[]);
    click_in(application, &plugins_title, 0);
    let owner = taide_native_ui::settings_view::Owner {
        project: alpha.clone(),
        pane: auxiliary_pane.clone(),
        tab: settings.id.clone(),
    };
    assert_eq!(
        application.settings_state.settings_views.inspection()[&owner].active,
        taide_native_ui::settings_controls::Section::Plugins
    );

    viewports.borrow_mut().close.insert(viewport);
    wait(
        application,
        "tabs return to the main window",
        &|application, _| {
            let layouts = application.services.state.layouts.read();
            layouts[&alpha].auxiliary_windows.is_empty()
                && crate::tabs::tabs_in(&layouts[&alpha].root)
                    .iter()
                    .any(|tab| tab.id == settings.id)
        },
    );
    let main_pane = pane_holding(&services.state.layouts.read()[&alpha].root, &settings.id)
        .expect("settings returned to the main window");
    application
        .controller
        .submit(ShellMutation::ActivateProject(alpha.clone()))
        .unwrap();
    application
        .controller
        .submit(ShellMutation::ActivateTab(settings.id.clone()))
        .unwrap();
    let main_owner = taide_native_ui::settings_view::Owner {
        project: alpha.clone(),
        pane: main_pane,
        tab: settings.id.clone(),
    };
    wait(
        application,
        "settings mounted again in the main window",
        &|application, _| {
            application
                .settings_state
                .settings_views
                .inspection()
                .contains_key(&main_owner)
        },
    );
    let views = application.settings_state.settings_views.inspection();
    assert!(
        !views.contains_key(&owner),
        "the auxiliary owner is dropped with its window"
    );
    assert_eq!(
        views[&main_owner].active,
        taide_native_ui::settings_controls::Section::Appearance
    );
}
