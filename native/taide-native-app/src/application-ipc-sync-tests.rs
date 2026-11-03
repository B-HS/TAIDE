use std::path::PathBuf;
use std::time::Duration;

use super::notifications_tests::{FakePlatform, frame, frame_until, launch_with_platform};
use super::*;
use crate::application::project_rail_tests::{
    Fixture, insert_project, isolated_state, project_ref,
};
use taide_model::app_event::AppEvent;
use taide_model::layout::{PaneNode, Tab};
use taide_runtime::AppState;

const SYNC_THEME: &str = "synthetic-sync-theme";
const FIRST_BACKGROUND: &str = "#101010";
const SYNCED_BACKGROUND: &str = "#202020";
const QUIET_FRAMES: usize = 8;
const FILE_NAME: &str = "notes.txt";
const DISK_TEXT: &str = "disk";

pub(super) struct Opened {
    pub(super) fixture: Fixture,
    pub(super) context: egui::Context,
    pub(super) project: ProjectId,
    pub(super) root: PathBuf,
    pub(super) paths: Vec<String>,
}

/// One project whose main pane holds a file tab for each `files` entry, focused in its slot.
pub(super) fn open_project(name: &str, files: &[(&str, &[u8])]) -> Opened {
    open_project_with(name, files, |_| {})
}

pub(super) fn open_project_with(
    name: &str,
    files: &[(&str, &[u8])],
    setup: impl FnOnce(&AppState),
) -> Opened {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-providers-{name}-{}",
            ProjectId::new()
        )),
        application: None,
    };
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(fixture.directory.join("data")).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let paths = files
        .iter()
        .map(|(file, bytes)| {
            let path = root.join(file);
            std::fs::write(&path, bytes).unwrap();
            path.to_str().unwrap().to_owned()
        })
        .collect::<Vec<_>>();
    let state = isolated_state(fixture.directory.join("data"));
    let project = ProjectId::new();
    insert_project(&state, &project, &root, "alpha", false);
    let leaves = paths
        .iter()
        .zip(files)
        .map(|(path, (file, _))| {
            let tab = Tab {
                id: TabId::new(),
                kind: TabKind::File { path: path.clone() },
                title: (*file).into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            };
            PaneNode::Leaf {
                id: PaneId::new(),
                active: Some(tab.id.clone()),
                tabs: vec![tab],
            }
        })
        .collect::<Vec<_>>();
    if let Some(layout) = state.layouts.write().get_mut(&project)
        && let Some(PaneNode::Leaf { id, .. }) = leaves.first()
    {
        layout.focused_pane = id.clone();
        layout.root = PaneNode::Split {
            id: PaneId::new(),
            dir: taide_model::layout::SplitDir::Horizontal,
            sizes: vec![1.0 / leaves.len() as f32; leaves.len()],
            children: leaves,
        };
    }
    {
        let slot = taide_model::ids::ShellSlotId::new();
        let mut session = state.session.write();
        session.projects = vec![project_ref(&project, &root, "alpha")];
        session.active_project = Some(project.clone());
        session.focused_shell_slot = Some(slot.clone());
        session.shell_slots = Some(taide_model::project::ShellSlotTree::Leaf {
            slot_id: slot,
            project_id: project.clone(),
        });
    }
    setup(&state);
    let context = launch_with_platform(&mut fixture, state, Arc::new(FakePlatform::default()));
    let application = fixture.application.as_mut().unwrap();
    frame_until(application, &context, true, "documents", |application| {
        paths
            .iter()
            .all(|path| application.files.contains_key(path))
    });
    Opened {
        fixture,
        context,
        project,
        root,
        paths,
    }
}

pub(super) fn edit(application: &mut NativeApplication, path: &str, text: &str) -> u64 {
    let document = application.files[path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap()
        .id;
    taide_native_editor::editing::replace_selections(&mut application.store, view, text, None)
        .unwrap();
    application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .revision
}

fn theme_json(background: &str) -> String {
    serde_json::json!({
        "version": 1,
        "id": SYNC_THEME,
        "name": "Synthetic sync",
        "type": "dark",
        "colors": { "app.background": background },
    })
    .to_string()
}

fn background(application: &NativeApplication) -> Option<&str> {
    application
        .settings_state
        .resolved_theme
        .colors
        .get("app.background")
        .map(String::as_str)
}

#[test]
fn 실제_app은_sync_적용_뒤_같은_테마의_바뀐_파일을_다시_읽는다() {
    let opened = open_project_with("sync-theme", &[], |state| {
        let file = state.paths.themes_dir().join(format!("{SYNC_THEME}.json"));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, theme_json(FIRST_BACKGROUND)).unwrap();
        let mut settings = state.settings.write();
        settings.follow_system_theme = false;
        settings.theme_id = SYNC_THEME.into();
    });
    let Opened {
        mut fixture,
        context,
        ..
    } = opened;
    let application = fixture.application.as_mut().unwrap();
    frame_until(application, &context, true, "first theme", |application| {
        background(application) == Some(FIRST_BACKGROUND)
    });
    let file = application
        .services
        .state
        .paths
        .themes_dir()
        .join(format!("{SYNC_THEME}.json"));
    std::fs::write(file, theme_json(SYNCED_BACKGROUND)).unwrap();
    for _ in 0..QUIET_FRAMES {
        frame(application, &context, true);
    }
    assert_eq!(background(application), Some(FIRST_BACKGROUND));
    application
        .services
        .events
        .publish(AppEvent::SyncStateChanged {
            status: taide_model::sync::SyncStatus {
                connected: true,
                has_gist: true,
                last_synced_at: None,
                remote_newer: None,
            },
        });
    frame_until(application, &context, true, "synced theme", |application| {
        background(application) == Some(SYNCED_BACKGROUND)
    });
}

#[test]
fn 실제_app은_앱_밖에서_닫힌_프로젝트의_문서와_탐색기_캐시를_비운다() {
    let Opened {
        mut fixture,
        context,
        project,
        paths,
        ..
    } = open_project("remote-close", &[(FILE_NAME, DISK_TEXT.as_bytes())]);
    let application = fixture.application.as_mut().unwrap();
    frame_until(
        application,
        &context,
        true,
        "explorer tree",
        |application| {
            application
                .explorer_search_state
                .trees
                .contains_key(&project)
        },
    );
    edit(application, &paths[0], "draft ");
    frame(application, &context, true);
    let services = application.services.clone();
    application
        .runtime
        .block_on(taide_runtime::project_actions::project_close(
            services.events.as_ref(),
            &services.state,
            &crate::projects::NativeProjects::new(services.clone()),
            project.clone(),
        ))
        .unwrap();
    frame_until(application, &context, true, "released", |application| {
        !application.files.contains_key(&paths[0])
    });
    let explorer = &application.explorer_search_state;
    assert!(!explorer.trees.contains_key(&project));
    assert!(!explorer.explorers.contains_key(&project));
    assert!(!explorer.tree_loading.contains(&project));
    assert!(
        application
            .drafts()
            .unwrap()
            .iter()
            .all(|draft| draft.project != project)
    );
}

#[test]
fn 실제_app은_rail_닫기_뒤_다시_연_프로젝트를_새_tree와_빈_선택으로_시작한다() {
    let Opened {
        mut fixture,
        context,
        project,
        root,
        paths,
    } = open_project("reopen", &[(FILE_NAME, DISK_TEXT.as_bytes())]);
    let application = fixture.application.as_mut().unwrap();
    frame_until(
        application,
        &context,
        true,
        "explorer tree",
        |application| {
            application
                .explorer_search_state
                .trees
                .contains_key(&project)
                && application
                    .explorer_search_state
                    .explorers
                    .contains_key(&project)
        },
    );
    let rows = application.explorer_search_state.trees[&project]
        .rows
        .clone();
    if let Some(explorer) = application
        .explorer_search_state
        .explorers
        .get_mut(&project)
    {
        explorer.select_path(&rows, &paths[0]);
        assert!(!explorer.selected_paths().is_empty());
    }
    application.close_project(project.clone());
    frame_until(application, &context, true, "closed", |application| {
        !application
            .services
            .state
            .projects
            .read()
            .contains_key(&project)
            && !application.files.contains_key(&paths[0])
    });
    assert!(
        !application
            .explorer_search_state
            .explorers
            .contains_key(&project)
    );
    assert!(
        !application
            .explorer_search_state
            .trees
            .contains_key(&project)
    );
    application.submit(HostCommand::Shell(Command::OpenProjectByPath(
        root.to_str().unwrap().into(),
    )));
    frame_until(
        application,
        &context,
        true,
        "reopened tree",
        |application| {
            let reopened = application
                .services
                .state
                .projects
                .read()
                .values()
                .find(|candidate| std::path::Path::new(&candidate.root) == root)
                .map(|candidate| candidate.id.clone());
            reopened.is_some_and(|reopened| {
                application
                    .explorer_search_state
                    .trees
                    .contains_key(&reopened)
            })
        },
    );
    assert!(
        application
            .explorer_search_state
            .explorers
            .values()
            .all(|explorer| explorer.selected_paths().is_empty())
    );
}

#[test]
fn 실제_app은_전송중_mirror보다_새_본문을_닫기_전에_mirror한다() {
    let Opened {
        mut fixture,
        context,
        project,
        paths,
        ..
    } = open_project("closing-mirror", &[(FILE_NAME, DISK_TEXT.as_bytes())]);
    let application = fixture.application.as_mut().unwrap();
    let document = application.files[&paths[0]].id;
    let first = edit(application, &paths[0], "first ");
    application
        .persistence
        .changed(document, first, Instant::now(), true, Duration::ZERO);
    let snapshot = application.store.documents().snapshot(document).unwrap();
    let inflight = application
        .persistence
        .closing_mirror_job(snapshot, project.clone())
        .unwrap();
    application.persistence.mirror_started(&inflight);
    let latest = edit(application, &paths[0], "second ");
    application
        .persistence
        .changed(document, latest, Instant::now(), true, Duration::ZERO);
    let expected = application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string();
    application.close_project(project.clone());
    frame_until(application, &context, true, "closed", |application| {
        !application
            .services
            .state
            .projects
            .read()
            .contains_key(&project)
    });
    let mirrors =
        taide_file::service::list_mirrors(&application.services.state.paths, &project).unwrap();
    assert!(
        mirrors
            .iter()
            .any(|mirror| mirror.path == paths[0] && mirror.content == expected),
        "{mirrors:?}"
    );
}
