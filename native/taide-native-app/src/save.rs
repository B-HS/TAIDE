use taide_model::{
    ids::TabId,
    layout::{Tab, TabKind},
};
use taide_native_editor::document::{DocumentId, DocumentKey, EditorError};
pub use taide_native_editor::save_preparation::{PreparedSave, prepare};
use taide_native_editor::store::{EditorStore, SaveSnapshot};

pub enum KeymapSave {
    File {
        path: String,
        snapshot: SaveSnapshot,
    },
    Untitled(TabId),
}

pub fn keymap_request(
    tab: &Tab,
    document: Option<DocumentId>,
    store: &mut EditorStore,
) -> Result<Option<KeymapSave>, EditorError> {
    let Some(document) = document else {
        return Ok(None);
    };
    match &tab.kind {
        TabKind::File { path } => {
            if !matches!(
                store.documents().snapshot(document)?.key,
                DocumentKey::File(_)
            ) {
                return Err(EditorError::InvalidIdentity);
            }
            Ok(Some(KeymapSave::File {
                path: path.clone(),
                snapshot: store.save_snapshot(document)?,
            }))
        }
        TabKind::Untitled { .. } => {
            let current = store.documents().snapshot(document)?;
            if current.key != DocumentKey::Untitled(tab.id.clone()) {
                return Err(EditorError::InvalidIdentity);
            }
            Ok(Some(KeymapSave::Untitled(tab.id.clone())))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{HostBridge, HostCommand, HostReply};
    use eframe::egui::{self, Color32, Event, FontId, ImeEvent, Key};
    use std::{path::PathBuf, sync::Arc, time::Duration};
    use taide_model::{
        app_event::AppEvent,
        ids::{PaneId, ProjectId, ShellSlotId},
        paths::AppPaths,
        project::{Project, ShellSlotTree},
    };
    use taide_native_editor::save_cleanup::CleanupFlags;
    use taide_native_editor::{store::EditorLimits, view::ViewKey};
    use taide_native_ui::{
        commands::ShellIntent,
        editor_surface::{EditorAppearance, NativeEditor},
        snapshot::ShellSnapshot,
    };
    use taide_runtime::{AppState, EventSink, TaskSupervisor};
    use tokio::sync::Notify;

    const DEADLINE: Duration = Duration::from_secs(3);
    const SCREEN: [f32; 2] = [640.0, 240.0];
    const FONT: f32 = 14.0;
    const LINE: f32 = 20.0;
    const PADDING: f32 = 8.0;
    const LIMIT: usize = 1024;
    const HISTORY: usize = 8;

    struct Sink;
    impl EventSink for Sink {
        fn publish(&self, _: AppEvent) {}
    }
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    async fn reply(bridge: &mut HostBridge, ready: &Notify) -> HostReply {
        tokio::time::timeout(DEADLINE, async {
            loop {
                if let Some(reply) = bridge.poll() {
                    return reply;
                }
                ready.notified().await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn save_keymap은_재지정_실제_editor와_등록문서_host_저장을_연결한다() {
        let directory =
            Directory(std::env::temp_dir().join(format!("taide-keymap-save-{}", ProjectId::new())));
        let root = directory.0.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("editor.txt");
        std::fs::write(&path, "draft").unwrap();
        let path = std::fs::canonicalize(path)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let project = ProjectId::new();
        let slot = ShellSlotId::new();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic save keymap".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        {
            let mut session = state.session.write();
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            });
            session.focused_shell_slot = Some(slot);
        }
        let tab = Tab {
            id: TabId::new(),
            kind: TabKind::File { path: path.clone() },
            title: "editor.txt".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        };
        let mut layout = taide_layout::service::default_layout();
        layout.root = taide_model::layout::PaneNode::Leaf {
            id: layout.focused_pane.clone(),
            tabs: vec![tab.clone()],
            active: Some(tab.id.clone()),
        };
        state.layouts.write().insert(project.clone(), layout);
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(Sink));
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge =
            HostBridge::connect(services, Arc::new(move || signal.notify_one())).unwrap();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: HISTORY,
            max_views: HISTORY,
            max_undo_groups: HISTORY,
            max_document_bytes: LIMIT,
        })
        .unwrap();
        bridge
            .submit(HostCommand::OpenDocument(path.clone()))
            .unwrap();
        let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
            panic!("expected document")
        };
        let document = result.unwrap().commit(&mut store).unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "main".into(),
                    pane: PaneId::new(),
                    tab: tab.id.clone(),
                },
                document,
            )
            .unwrap();
        let editor = NativeEditor {
            appearance: EditorAppearance {
                font: FontId::monospace(FONT),
                line_height: LINE,
                horizontal_padding: PADDING,
                background: Color32::BLACK,
                foreground: Color32::WHITE,
                muted: Color32::GRAY,
                selection: Color32::GRAY,
                cursor: Color32::WHITE,
                current_line: Color32::BLACK,
                line_numbers: true,
                indent: "\t".into(),
            },
        };
        let context = egui::Context::default();
        let mut views = crate::terminal_surface::Views::default();
        let mut actions = Vec::new();
        let command = if cfg!(target_os = "macos") {
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL | egui::Modifiers::COMMAND
        };
        let key = |key, modifiers, pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        };
        {
            let mut draw = |mut events: Vec<Event>, overrides: Option<&str>| {
                let releases = events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Key {
                            key: pressed,
                            modifiers,
                            pressed: true,
                            ..
                        } => Some(key(*pressed, *modifiers, false)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                events.extend(releases);
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(SCREEN[0], SCREEN[1]),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        let mut next = 0;
                        let output = editor
                            .show_with_keymap(ui, &mut store, view, true, |ui, event, composing| {
                                let index = crate::keymap::event_index(ui.ctx(), event, &mut next);
                                views
                                    .route_keymap(
                                        crate::keymap::Route {
                                            context: ui.ctx(),
                                            event,
                                            index,
                                            scope: crate::keymap::Context {
                                                editor: true,
                                                terminal: false,
                                            },
                                            composing,
                                            overrides,
                                        },
                                        &mut actions,
                                        true,
                                    )
                                    .unwrap()
                            })
                            .unwrap();
                        assert!(!output.save_requested);
                        assert!(output.errors.is_empty());
                        if ui.ctx().current_pass_index() == 0 {
                            ui.ctx().request_discard("synthetic save keymap multi-pass");
                        }
                    },
                );
                output.textures_delta.clear();
            };
            draw(vec![key(Key::S, command, true)], None);
            let rebound = Some(r#"[{"actionId":"save","key":"j","mods":[]}]"#);
            draw(vec![key(Key::S, command, true)], rebound);
            draw(
                vec![
                    key(Key::J, Default::default(), true),
                    Event::Text("j".into()),
                ],
                rebound,
            );
            draw(
                vec![key(Key::S, command | egui::Modifiers::SHIFT, true)],
                None,
            );
            draw(
                vec![
                    Event::Ime(ImeEvent::Preedit {
                        text: "漢".into(),
                        active_range_chars: None,
                    }),
                    key(Key::J, Default::default(), true),
                    Event::Ime(ImeEvent::Commit("漢".into())),
                ],
                rebound,
            );
        }
        assert_eq!(actions, ["save", "save"]);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "漢draft"
        );
        let shell = ShellSnapshot::read(&state).await;
        assert!(
            matches!(crate::shell_keymap::action("save", &shell), Some(ShellIntent::RequestSaveTab(id)) if id == tab.id)
        );
        assert!(keymap_request(&tab, None, &mut store).unwrap().is_none());
        let Some(KeymapSave::File {
            path: target,
            snapshot,
        }) = keymap_request(&tab, Some(document), &mut store).unwrap()
        else {
            panic!("expected registered file save")
        };
        assert_eq!(target, path);
        let prepared = prepare(
            &mut store,
            snapshot,
            Some(view),
            CleanupFlags {
                trim_trailing_whitespace: false,
                insert_final_newline: false,
            },
            false,
        )
        .unwrap()
        .unwrap();
        bridge
            .submit(HostCommand::Save {
                path: target,
                snapshot: prepared.snapshot,
            })
            .unwrap();
        let HostReply::Saved { snapshot, result } = reply(&mut bridge, &ready).await else {
            panic!("expected file save")
        };
        let written = result.unwrap();
        assert!(
            store
                .mark_saved(snapshot, Some(written.modified_ms))
                .unwrap()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "漢draft");
        let mut readonly = written;
        readonly.read_only = true;
        store
            .observe_file(document, std::path::Path::new(&path), readonly)
            .unwrap();
        assert!(matches!(
            keymap_request(&tab, Some(document), &mut store),
            Err(EditorError::ReadOnly)
        ));
        let mut untitled_tab = tab.clone();
        untitled_tab.kind = TabKind::Untitled { index: 1 };
        let untitled = store
            .open_untitled(tab.id.clone(), "untitled", "plaintext".into())
            .unwrap();
        assert!(
            matches!(keymap_request(&untitled_tab, Some(untitled), &mut store).unwrap(), Some(KeymapSave::Untitled(id)) if id == tab.id)
        );
        assert!(matches!(
            keymap_request(&tab, Some(untitled), &mut store),
            Err(EditorError::InvalidIdentity)
        ));
        untitled_tab.id = TabId::new();
        assert!(matches!(
            keymap_request(&untitled_tab, Some(untitled), &mut store),
            Err(EditorError::InvalidIdentity)
        ));
        let mut terminal = tab.clone();
        terminal.kind = TabKind::Terminal {
            session_id: "synthetic".into(),
            cwd: None,
        };
        assert!(
            keymap_request(&terminal, Some(document), &mut store)
                .unwrap()
                .is_none()
        );
        let mut terminal_shell = shell;
        let taide_model::layout::PaneNode::Leaf { tabs, .. } =
            &mut terminal_shell.layouts.get_mut(&project).unwrap().root
        else {
            panic!("expected leaf")
        };
        tabs[0] = terminal;
        assert!(crate::shell_keymap::action("save", &terminal_shell).is_none());
        bridge.disconnect().await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }
}
