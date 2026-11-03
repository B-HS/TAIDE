use std::path::PathBuf;

use super::notifications_tests::{frame, frame_until};
use super::*;
use crate::application::project_rail_tests::{Fixture, isolated_state, launch};
use taide_model::agent::ExternalOpenRequest;
use taide_model::app_event::AppEvent;
use taide_model::layout::{PaneNode, Tab};
use taide_native_ui::toast::Kind;

const HALF_SPLIT: f32 = 0.5;
const SETTLE_FRAMES: usize = 4;

struct Marker(PathBuf);

impl Marker {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("taide-wait-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, "").unwrap();
        Self(path)
    }

    fn text(&self) -> String {
        self.0.to_str().unwrap().to_owned()
    }
}

impl Drop for Marker {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn request(application: &NativeApplication, path: &std::path::Path, marker: Option<&Marker>) {
    let request = ExternalOpenRequest {
        path: path.to_str().unwrap().into(),
        wait_marker: marker.map(Marker::text),
    };
    crate::external_open::queue(&application.services, request.clone());
    application
        .services
        .events
        .publish(AppEvent::AgentExternalOpen { request });
}

fn project_at(application: &NativeApplication, root: &std::path::Path) -> Option<ProjectId> {
    let root = root.to_str().unwrap();
    application
        .services
        .state
        .projects
        .read()
        .values()
        .find(|project| project.root == root)
        .map(|project| project.id.clone())
}

fn file_tabs(application: &NativeApplication, path: &str) -> Vec<(ProjectId, Tab)> {
    application
        .services
        .state
        .layouts
        .read()
        .iter()
        .flat_map(|(project, layout)| {
            taide_layout::service::all_roots(layout)
                .flat_map(crate::tabs::tabs_in)
                .filter(|tab| matches!(&tab.kind, TabKind::File { path: open } if open == path))
                .map(|tab| (project.clone(), tab.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn has_toast(application: &NativeApplication, kind: Kind, key: Option<&str>) -> bool {
    let title = key.map(|key| presentation::message(&application.locale, key, &[]));
    application
        .toasts
        .inspection()
        .iter()
        .any(|toast| toast.kind == kind && title.as_ref().is_none_or(|title| &toast.title == title))
}

fn close(application: &mut NativeApplication, context: &egui::Context, tab: &Tab) {
    application.submit(HostCommand::Editor(
        crate::domains::editor::Command::CloseTab {
            tab: tab.id.clone(),
            discard: false,
        },
    ));
    let path = match &tab.kind {
        TabKind::File { path } => path.clone(),
        _ => String::new(),
    };
    let id = tab.id.clone();
    frame_until(application, context, true, "tab closed", |application| {
        file_tabs(application, &path)
            .iter()
            .all(|(_, open)| open.id != id)
    });
}

#[test]
fn 실제_app은_외부_열기_요청을_프로젝트_탭_wait_marker와_안내로_처리한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-external-open-{}", ProjectId::new())),
        application: None,
    };
    std::fs::create_dir_all(fixture.directory.join("data")).unwrap();
    let base = fixture.directory.canonicalize().unwrap();
    let alpha = base.join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    let notes = alpha.join("notes.md");
    std::fs::write(&notes, "# notes").unwrap();
    let loose = base.join("loose.md");
    std::fs::write(&loose, "loose").unwrap();
    let state = isolated_state(fixture.directory.join("data"));
    let context = launch(&mut fixture, state);
    let cold = Marker::new();
    let application =
        fixture
            .application
            .take()
            .unwrap()
            .with_external_open(Some(ExternalOpenRequest {
                path: loose.to_str().unwrap().into(),
                wait_marker: Some(cold.text()),
            }));
    fixture.application = Some(application);
    let application = fixture.application.as_mut().unwrap();
    frame_until(
        application,
        &context,
        true,
        "cold start drain",
        |application| has_toast(application, Kind::Info, Some("app.openProjectFirst")),
    );
    frame_until(application, &context, true, "cold marker", |_| {
        !cold.0.exists()
    });

    let opened = Marker::new();
    request(application, &alpha, Some(&opened));
    frame_until(
        application,
        &context,
        true,
        "project opened",
        |application| project_at(application, &alpha).is_some() && !opened.0.exists(),
    );
    let project = project_at(application, &alpha).unwrap();
    assert!(file_tabs(application, notes.to_str().unwrap()).is_empty());

    let missing = Marker::new();
    request(application, &alpha.join("missing.md"), Some(&missing));
    frame_until(application, &context, true, "missing file", |application| {
        has_toast(application, Kind::Error, None) && !missing.0.exists()
    });

    let waiting = Marker::new();
    let notes_path = notes.to_str().unwrap().to_owned();
    request(application, &notes, Some(&waiting));
    frame_until(application, &context, true, "wait tab", |application| {
        !file_tabs(application, &notes_path).is_empty()
            && has_toast(application, Kind::Info, Some("app.externalEditorTabHint"))
    });
    let (owner, tab) = file_tabs(application, &notes_path).remove(0);
    assert_eq!(owner, project);
    assert!(!tab.preview);
    assert!(waiting.0.exists());
    let twin = Tab {
        id: TabId::new(),
        ..tab.clone()
    };
    if let Some(layout) = application.services.state.layouts.write().get_mut(&project) {
        let root = layout.root.clone();
        layout.root = PaneNode::Split {
            id: PaneId::new(),
            dir: taide_model::layout::SplitDir::Horizontal,
            children: vec![
                root,
                PaneNode::Leaf {
                    id: PaneId::new(),
                    tabs: vec![twin.clone()],
                    active: Some(twin.id.clone()),
                },
            ],
            sizes: vec![HALF_SPLIT, HALF_SPLIT],
        };
    }
    close(application, &context, &tab);
    for _ in 0..SETTLE_FRAMES {
        frame(application, &context, true);
    }
    assert!(waiting.0.exists());
    close(application, &context, &twin);
    frame_until(application, &context, true, "wait marker released", |_| {
        !waiting.0.exists()
    });

    let ide_waiting = Marker::new();
    request(application, &notes, Some(&ide_waiting));
    frame_until(application, &context, true, "ide wait tab", |application| {
        !file_tabs(application, &notes_path).is_empty()
    });
    let (_, tab) = file_tabs(application, &notes_path).remove(0);
    assert!(ide_waiting.0.exists());
    let services = application.services.clone();
    let actions = application.application_ports.ide.actions.clone();
    let arguments = serde_json::json!({ "tab_name": tab.title });
    let closed = application.runtime.spawn(async move {
        crate::ide_tools::dispatch(services, &actions, "close_tab", &arguments).await
    });
    frame_until(application, &context, true, "ide close_tab", |_| {
        closed.is_finished()
    });
    assert!(application.runtime.block_on(closed).unwrap().is_ok());
    assert!(file_tabs(application, &notes_path).is_empty());
    frame_until(
        application,
        &context,
        true,
        "ide close released marker",
        |_| !ide_waiting.0.exists(),
    );

    let loose_path = loose.to_str().unwrap().to_owned();
    request(application, &loose, None);
    frame_until(application, &context, true, "outside file", |application| {
        !file_tabs(application, &loose_path).is_empty()
    });
    let (owner, tab) = file_tabs(application, &loose_path).remove(0);
    assert_eq!(owner, project);
    assert!(tab.preview);

    application
        .shell_state
        .provider_events
        .request_main_window();
    let mut output = context.run_ui(egui::RawInput::default(), |ui| {
        eframe::App::logic(application, ui.ctx(), &mut eframe::Frame::_new_kittest());
        eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest());
    });
    output.textures_delta.clear();
    let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
    assert!(commands.contains(&egui::ViewportCommand::Visible(true)));
    assert!(commands.contains(&egui::ViewportCommand::Focus));
}
