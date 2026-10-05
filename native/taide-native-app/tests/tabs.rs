use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{AuxWindowLayout, PaneNode, Tab, TabKind};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::close_dialog::{self, CloseChoice};
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_runtime::{AppState, EventSink, TaskSupervisor, file_actions, layout_actions};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(3);
const AUX_SLOT: u32 = 1;
const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;
const FRAME_TIME: f64 = 0.1;

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    dir: PathBuf,
    path: String,
    id: ProjectId,
    state: AppState,
}
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-native-tabs-{}", ProjectId::new()));
        std::fs::create_dir_all(dir.join("root")).unwrap();
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let path = dir.join("root/main.rs");
        std::fs::write(&path, "disk").unwrap();
        let id = ProjectId::new();
        let state = AppState::new(AppPaths::new(dir.join("data")));
        state.projects.write().insert(
            id.clone(),
            Project {
                id: id.clone(),
                root: dir.join("root").to_str().unwrap().into(),
                name: "synthetic close".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        state
            .layouts
            .write()
            .insert(id.clone(), taide_layout::service::default_layout());
        Self {
            dir,
            path: path.to_str().unwrap().into(),
            id,
            state,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

fn find(state: &AppState, project: &ProjectId, path: &str) -> Tab {
    let layouts = state.layouts.read();
    taide_layout::service::all_roots(&layouts[project])
        .flat_map(taide_native_app::tabs::tabs_in)
        .find(|tab| matches!(&tab.kind, TabKind::File { path: current } if current == path))
        .unwrap()
        .clone()
}

async fn reply(bridge: &mut HostBridge, signal: &Notify) -> HostReply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            signal.notified().await;
        }
    })
    .await
    .unwrap()
}

#[test]
fn 실제_host_닫기는_pinned_dirty와_공유_mirror를_보호하고_폐기나_저장_후에만_회수한다() {
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = services(fixture.state.clone(), tasks.clone(), Arc::new(Sink));
    let signal = Arc::new(Notify::new());
    let ready = signal.clone();
    let mut bridge =
        HostBridge::connect(services.clone(), Arc::new(move || ready.notify_one())).unwrap();
    runtime.block_on(async {
        layout_actions::layout_open_tab(
            services.events.as_ref(),
            &fixture.state,
            fixture.id.clone(),
            TabKind::File {
                path: fixture.path.clone(),
            },
            "main.rs".into(),
            None,
            false,
        )
        .await
        .unwrap();
        let tab = find(&fixture.state, &fixture.id, &fixture.path);
        layout_actions::layout_pin_tab(
            services.events.as_ref(),
            &fixture.state,
            tab.id.clone(),
            true,
        )
        .await
        .unwrap();
        bridge
            .submit(HostCommand::CloseTab {
                tab: tab.id.clone(),
                discard: true,
            })
            .unwrap();
        assert!(matches!(
            reply(&mut bridge, &signal).await,
            HostReply::Closed { result: Err(_), .. }
        ));
        assert_eq!(find(&fixture.state, &fixture.id, &fixture.path).id, tab.id);
        layout_actions::layout_pin_tab(
            services.events.as_ref(),
            &fixture.state,
            tab.id.clone(),
            false,
        )
        .await
        .unwrap();
        file_actions::file_mirror_dirty(
            &fixture.state,
            &tasks,
            fixture.id.clone(),
            fixture.path.clone(),
            "saved draft".into(),
        )
        .await
        .unwrap();
        layout_actions::layout_set_dirty(
            services.events.as_ref(),
            &fixture.state,
            tab.id.clone(),
            true,
        )
        .await
        .unwrap();
        bridge
            .submit(HostCommand::CloseTab {
                tab: tab.id.clone(),
                discard: false,
            })
            .unwrap();
        assert!(matches!(
            reply(&mut bridge, &signal).await,
            HostReply::Closed { result: Err(_), .. }
        ));
        assert_eq!(
            file_actions::file_list_mirrors(&fixture.state, fixture.id.clone())
                .await
                .unwrap()
                .len(),
            1
        );
        let mut shared = tab.clone();
        shared.id = TabId::new();
        shared.dirty = true;
        let auxiliary = taide_layout::service::default_layout();
        let pane = auxiliary.focused_pane.clone();
        let PaneNode::Leaf { id, .. } = auxiliary.root else {
            panic!("pane")
        };
        fixture
            .state
            .layouts
            .write()
            .get_mut(&fixture.id)
            .unwrap()
            .auxiliary_windows
            .push(AuxWindowLayout {
                slot: AUX_SLOT,
                root: PaneNode::Leaf {
                    id,
                    tabs: vec![shared.clone()],
                    active: Some(shared.id.clone()),
                },
                focused_pane: pane,
            });
        bridge
            .submit(HostCommand::CloseTab {
                tab: tab.id.clone(),
                discard: true,
            })
            .unwrap();
        let HostReply::Closed { result, .. } = reply(&mut bridge, &signal).await else {
            panic!("close reply")
        };
        let closed = result.unwrap();
        assert!(closed.has_remaining_file);
        assert!(!closed.tab.dirty);
        assert_eq!(
            file_actions::file_list_mirrors(&fixture.state, fixture.id.clone())
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(find(&fixture.state, &fixture.id, &fixture.path).dirty);
        bridge
            .submit(HostCommand::SaveMirroredTab(shared.id.clone()))
            .unwrap();
        assert!(matches!(
            reply(&mut bridge, &signal).await,
            HostReply::MirrorSaved { result: Ok(()), .. }
        ));
        assert_eq!(
            std::fs::read_to_string(&fixture.path).unwrap(),
            "saved draft"
        );
        bridge
            .submit(HostCommand::SetDirty {
                tab: shared.id.clone(),
                dirty: false,
            })
            .unwrap();
        bridge
            .submit(HostCommand::CloseTab {
                tab: shared.id,
                discard: false,
            })
            .unwrap();
        let HostReply::Closed { result, .. } = reply(&mut bridge, &signal).await else {
            panic!("close reply")
        };
        assert!(!result.unwrap().has_remaining_file);
        assert!(
            file_actions::file_list_mirrors(&fixture.state, fixture.id.clone())
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            fixture.state.layouts.read()[&fixture.id]
                .closed_tabs
                .iter()
                .all(|closed| !closed.tab.dirty)
        );
        bridge.disconnect().await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn 저장_실패는_mirror와_탭을_남기고_modal_escape는_cancel이다() {
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = services(fixture.state.clone(), tasks.clone(), Arc::new(Sink));
    runtime.block_on(async {
        layout_actions::layout_open_tab(
            services.events.as_ref(),
            &fixture.state,
            fixture.id.clone(),
            TabKind::File {
                path: fixture.path.clone(),
            },
            "main.rs".into(),
            None,
            false,
        )
        .await
        .unwrap();
        let tab = find(&fixture.state, &fixture.id, &fixture.path);
        file_actions::file_mirror_dirty(
            &fixture.state,
            &tasks,
            fixture.id.clone(),
            fixture.path.clone(),
            "preserve".into(),
        )
        .await
        .unwrap();
        std::fs::remove_file(&fixture.path).unwrap();
        assert!(
            taide_native_app::tabs::save_mirrored(&services, tab.id.clone())
                .await
                .is_err()
        );
        assert_eq!(find(&fixture.state, &fixture.id, &fixture.path).id, tab.id);
        assert_eq!(
            file_actions::file_list_mirrors(&fixture.state, fixture.id.clone())
                .await
                .unwrap()[0]
                .content,
            "preserve"
        );
        let locale =
            taide_runtime::locale_actions::locale_get_current(&fixture.state, "en").unwrap();
        let context = egui::Context::default();
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(WIDTH, HEIGHT),
                )),
                ..Default::default()
            },
            |ui| {
                assert_eq!(close_dialog::show(ui.ctx(), &locale, &tab, false), None);
            },
        );
        output.textures_delta.clear();
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(FRAME_TIME),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(WIDTH, HEIGHT),
                )),
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| {
                assert_eq!(
                    close_dialog::show(ui.ctx(), &locale, &tab, false),
                    Some(CloseChoice::Cancel)
                );
            },
        );
        output.textures_delta.clear();
        assert_eq!(find(&fixture.state, &fixture.id, &fixture.path).id, tab.id);
        assert_eq!(tasks.tracked_count(), 0);
    });
}
