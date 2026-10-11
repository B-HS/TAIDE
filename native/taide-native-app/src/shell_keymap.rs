use taide_model::{
    ids::PaneId,
    layout::{DropEdge, PaneNode, SplitDir, TabKind},
};
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::snapshot::ShellSnapshot;

use crate::command_registry::{Direction, GroupTarget, Run, TabCycle, keymap_run};

fn leaves(node: &PaneNode) -> Vec<&PaneNode> {
    match node {
        PaneNode::Leaf { .. } => vec![node],
        PaneNode::Split { children, .. } => children.iter().flat_map(leaves).collect(),
    }
}

fn adjacent<'a>(node: &'a PaneNode, focused: &PaneId, edge: DropEdge) -> Option<&'a PaneNode> {
    let PaneNode::Split { dir, children, .. } = node else {
        return None;
    };
    let index = children
        .iter()
        .position(|child| taide_layout::service::find_leaf(child, focused).is_some())?;
    if let Some(nearest) = adjacent(&children[index], focused, edge) {
        return Some(nearest);
    }
    let (axis, forward) = match edge {
        DropEdge::Left => (SplitDir::Horizontal, false),
        DropEdge::Right => (SplitDir::Horizontal, true),
        DropEdge::Top => (SplitDir::Vertical, false),
        DropEdge::Bottom => (SplitDir::Vertical, true),
        DropEdge::Center => return None,
    };
    if *dir != axis {
        return None;
    }
    let sibling = if forward {
        index.checked_add(1)?
    } else {
        index.checked_sub(1)?
    };
    let candidates = leaves(children.get(sibling)?);
    if forward {
        candidates.first().copied()
    } else {
        candidates.last().copied()
    }
}

fn drop_edge(direction: Direction) -> DropEdge {
    match direction {
        Direction::Left => DropEdge::Left,
        Direction::Right => DropEdge::Right,
        Direction::Up => DropEdge::Top,
        Direction::Down => DropEdge::Bottom,
    }
}

pub(crate) fn supports(action: &str) -> bool {
    keymap_run(action).is_some()
}

pub(crate) fn runs_without_focused_project(action: &str) -> bool {
    matches!(
        keymap_run(action),
        Some(
            Run::OpenPalette(_)
                | Run::ToggleSidebar
                | Run::OpenKeybindingsEditor
                | Run::OpenTerminalTab
                | Run::ReopenClosedTab
                | Run::ChangeEditorFontSize { .. }
                | Run::ToggleZenMode
        )
    )
}

pub(crate) fn action(action: &str, snapshot: &ShellSnapshot) -> Option<ShellIntent> {
    intent(keymap_run(action)?, snapshot)
}

pub(crate) fn intent(run: Run, snapshot: &ShellSnapshot) -> Option<ShellIntent> {
    match run {
        Run::OpenPalette(entry) => return Some(ShellIntent::OpenPalette(entry)),
        Run::OpenKeybindingsEditor => return Some(ShellIntent::OpenKeybindings),
        Run::OpenSettingsTab => return Some(ShellIntent::OpenSettings),
        Run::OpenSettingsFile => return Some(ShellIntent::OpenSettingsFile),
        Run::ToggleZenMode => {
            return Some(ShellIntent::Mutate(ShellMutation::SetWindowChrome(
                taide_model::project::WindowChromePatch {
                    zen: Some(!snapshot.shell.window_chrome.zen),
                    ..Default::default()
                },
            )));
        }
        Run::ChangeEditorFontSize { increase } => {
            return Some(ShellIntent::ChangeEditorFontSize { increase });
        }
        Run::OpenTerminalTab if snapshot.focused_project().is_none() => {
            return Some(ShellIntent::ShowOpenProjectNotice);
        }
        _ => (),
    }
    let project = snapshot.focused_project()?;
    let layout = snapshot.layouts.get(project)?;
    match run {
        Run::ReopenClosedTab => {
            return Some(ShellIntent::Mutate(ShellMutation::ReopenClosed(
                project.clone(),
            )));
        }
        Run::ToggleSidebar => {
            if snapshot.shell.window_chrome.zen {
                return None;
            }
            return Some(ShellIntent::Mutate(ShellMutation::SetSidebarCollapsed {
                project: project.clone(),
                collapsed: !layout.shell_view.sidebar_collapsed,
            }));
        }
        Run::FocusGroup(target) => {
            let target = match target {
                GroupTarget::Direction(direction) => {
                    adjacent(&layout.root, &layout.focused_pane, drop_edge(direction))
                }
                GroupTarget::Position(position) => {
                    leaves(&layout.root).get(position.checked_sub(1)?).copied()
                }
            }?;
            let PaneNode::Leaf { id, .. } = target else {
                return None;
            };
            return (id != &layout.focused_pane)
                .then(|| ShellIntent::Mutate(ShellMutation::FocusPane(id.clone())));
        }
        _ => (),
    }
    let PaneNode::Leaf { tabs, active, .. } =
        taide_layout::service::find_leaf(&layout.root, &layout.focused_pane)?
    else {
        return None;
    };
    if run == Run::OpenTerminalTab {
        return Some(ShellIntent::NewTerminal {
            project: project.clone(),
            pane: layout.focused_pane.clone(),
        });
    }
    if run == Run::ToggleTerminal {
        let active_terminal = tabs.iter().any(|tab| {
            Some(&tab.id) == active.as_ref() && matches!(tab.kind, TabKind::Terminal { .. })
        });
        let target = tabs.iter().find(|tab| {
            matches!(tab.kind, TabKind::Terminal { .. }) != active_terminal
                && (!active_terminal || Some(&tab.id) != active.as_ref())
        });
        if let Some(target) = target {
            return Some(ShellIntent::Mutate(ShellMutation::ActivateTab(
                target.id.clone(),
            )));
        }
        return (!active_terminal).then(|| ShellIntent::NewTerminal {
            project: project.clone(),
            pane: layout.focused_pane.clone(),
        });
    }
    if run == Run::CloseAllTabs {
        return Some(ShellIntent::RequestCloseTabs(
            tabs.iter()
                .filter(|tab| !tab.pinned)
                .map(|tab| tab.id.clone())
                .collect(),
        ));
    }
    if let Run::MoveTabToGroup(direction) = run {
        let PaneNode::Leaf {
            id,
            tabs: target_tabs,
            ..
        } = adjacent(&layout.root, &layout.focused_pane, drop_edge(direction))?
        else {
            return None;
        };
        return Some(ShellIntent::Mutate(ShellMutation::MoveTab {
            tab: active.clone()?,
            pane: id.clone(),
            index: target_tabs.len().try_into().ok()?,
        }));
    }
    let current = tabs
        .iter()
        .position(|tab| Some(&tab.id) == active.as_ref())?;
    let tab = &tabs[current];
    let has_document = matches!(
        tab.kind,
        TabKind::File { .. } | TabKind::AppFile { .. } | TabKind::Untitled { .. }
    );
    match run {
        Run::SaveActiveTab if has_document => Some(ShellIntent::RequestSaveTab(tab.id.clone())),
        Run::ToggleEditorStickyScroll if has_document => {
            Some(ShellIntent::ToggleEditorStickyScroll)
        }
        Run::ToggleEditorMinimap if has_document => Some(ShellIntent::ToggleEditorMinimap),
        Run::ChooseIndentation(command) if has_document => Some(ShellIntent::ChooseIndentation {
            tab: tab.id.clone(),
            command,
        }),
        Run::FormatEditor(command) if has_document => Some(ShellIntent::FormatEditor {
            tab: tab.id.clone(),
            command,
        }),
        Run::RenameEditor if has_document => Some(ShellIntent::RenameEditor {
            tab: tab.id.clone(),
        }),
        Run::EditDocument(edit) if has_document => Some(ShellIntent::EditDocument {
            tab: tab.id.clone(),
            edit,
        }),
        Run::FoldDocument(command) if has_document => Some(ShellIntent::FoldDocument {
            tab: tab.id.clone(),
            command,
        }),
        Run::CloseTab => Some(ShellIntent::RequestCloseTab(tab.id.clone())),
        Run::Split => Some(ShellIntent::Mutate(ShellMutation::SplitTab {
            pane: layout.focused_pane.clone(),
            edge: DropEdge::Right,
            tab: tab.id.clone(),
        })),
        Run::CycleTab(TabCycle::Next) if tabs.len() > 1 => Some(ShellIntent::Mutate(
            ShellMutation::ActivateTab(tabs[(current + 1) % tabs.len()].id.clone()),
        )),
        Run::CycleTab(TabCycle::Previous) if tabs.len() > 1 => Some(ShellIntent::Mutate(
            ShellMutation::ActivateTab(tabs[(current + tabs.len() - 1) % tabs.len()].id.clone()),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Color32, Event, FontId, ImeEvent, Key};
    use std::sync::Arc;
    use std::time::Duration;
    use taide_model::{
        app_event::AppEvent,
        ids::{PaneId, ProjectId, ShellSlotId, TabId},
        paths::AppPaths,
        project::{ProjectRef, ShellSlotTree},
    };
    use taide_native_editor::{
        store::{EditorLimits, EditorStore},
        view::ViewKey,
    };
    use taide_native_ui::{
        controller::ShellController,
        editor_surface::{EditorAppearance, NativeEditor},
    };
    use taide_runtime::{AppState, EventSink, TaskSupervisor};

    const DEADLINE: Duration = Duration::from_secs(3);
    const SCREEN: [f32; 2] = [640.0, 240.0];
    const FONT_SIZE: f32 = 14.0;
    const LINE_HEIGHT: f32 = 20.0;
    const PADDING: f32 = 8.0;
    const DOCUMENT_BYTES: usize = 1024;
    const UNDO_GROUPS: usize = 8;
    struct Sink;
    impl EventSink for Sink {
        fn publish(&self, _: AppEvent) {}
    }

    #[tokio::test]
    async fn terminal_tab_keymap은_첫대상_빈pane_새탭과_격리host를_보존한다() {
        use crate::host::{HostBridge, HostCommand};
        use taide_model::layout::Tab;
        use tokio::sync::Notify;
        struct Directory(std::path::PathBuf);
        impl Drop for Directory {
            fn drop(&mut self) {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }
        let directory = Directory(
            std::env::temp_dir().join(format!("taide-terminal-tab-keymap-{}", ProjectId::new())),
        );
        std::fs::create_dir_all(&directory.0).unwrap();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        let project = ProjectId::new();
        let background = ProjectId::new();
        let other = taide_layout::service::default_layout();
        let mut layout = taide_layout::service::default_layout();
        let pane = layout.focused_pane.clone();
        let make_tab = |kind, pinned| Tab {
            id: TabId::new(),
            kind,
            title: "synthetic terminal keymap".into(),
            pinned,
            preview: false,
            dirty: false,
            view_state: None,
        };
        let first_terminal = make_tab(
            TabKind::Terminal {
                session_id: String::new(),
                cwd: None,
            },
            true,
        );
        let second_terminal = make_tab(first_terminal.kind.clone(), false);
        let first_editor = make_tab(TabKind::Untitled { index: 1 }, true);
        let second_editor = make_tab(TabKind::Untitled { index: 1 }, false);
        layout.root = PaneNode::Leaf {
            id: pane.clone(),
            active: Some(second_terminal.id.clone()),
            tabs: vec![
                first_terminal.clone(),
                first_editor.clone(),
                second_terminal.clone(),
                second_editor.clone(),
            ],
        };
        state.layouts.write().extend([
            (project.clone(), layout.clone()),
            (background.clone(), other.clone()),
        ]);
        let unfocused = ShellSnapshot::read(&state).await;
        assert!(matches!(
            action("new-terminal", &unfocused),
            Some(ShellIntent::ShowOpenProjectNotice)
        ));
        assert!(action("toggle-terminal", &unfocused).is_none());
        let slot = ShellSlotId::new();
        {
            let mut session = state.session.write();
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            });
            session.focused_shell_slot = Some(slot);
        }
        let snapshot = ShellSnapshot::read(&state).await;
        let activate = |snapshot: &ShellSnapshot| match action("toggle-terminal", snapshot) {
            Some(ShellIntent::Mutate(ShellMutation::ActivateTab(id))) => Some(id),
            None => None,
            _ => panic!("expected activation or no-op"),
        };
        assert_eq!(activate(&snapshot), Some(first_editor.id.clone()));
        let Some(ShellIntent::Mutate(mutation)) = action("toggle-terminal", &snapshot) else {
            panic!("expected editor fallback")
        };
        taide_native_ui::commands::dispatch(&Sink, &state, mutation)
            .await
            .unwrap();
        let snapshot = ShellSnapshot::read(&state).await;
        assert_eq!(snapshot.focused_tab().unwrap().id, first_editor.id);
        assert_eq!(activate(&snapshot), Some(first_terminal.id.clone()));
        let new_tab = |snapshot: &ShellSnapshot, key| {
            assert!(supports(key));
            assert!(
                matches!(action(key, snapshot), Some(ShellIntent::NewTerminal { project: target, pane: target_pane }) if target == project && target_pane == pane)
            );
        };
        new_tab(&snapshot, "new-terminal");
        let mut terminals_only = snapshot.clone();
        let PaneNode::Leaf { tabs, active, .. } =
            &mut terminals_only.layouts.get_mut(&project).unwrap().root
        else {
            panic!("expected leaf")
        };
        *tabs = vec![first_terminal.clone(), second_terminal.clone()];
        *active = Some(second_terminal.id.clone());
        assert!(activate(&terminals_only).is_none());
        new_tab(&terminals_only, "new-terminal");
        let PaneNode::Leaf { active, .. } =
            &mut terminals_only.layouts.get_mut(&project).unwrap().root
        else {
            panic!("expected leaf")
        };
        *active = Some(TabId::new());
        assert_eq!(activate(&terminals_only), Some(first_terminal.id.clone()));
        let mut empty = snapshot.clone();
        let PaneNode::Leaf { tabs, active, .. } =
            &mut empty.layouts.get_mut(&project).unwrap().root
        else {
            panic!("expected leaf")
        };
        tabs.clear();
        *active = None;
        new_tab(&empty, "toggle-terminal");
        new_tab(&empty, "new-terminal");
        let mut missing = empty.clone();
        missing.layouts.get_mut(&project).unwrap().focused_pane = PaneId::new();
        assert!(action("new-terminal", &missing).is_none());
        assert!(action("toggle-terminal", &missing).is_none());
        let now = std::time::Instant::now();
        let mut map = crate::keymap::Keymap::new().unwrap();
        for (shift, key, expected) in [(false, "`", "toggle-terminal"), (true, "~", "new-terminal")]
        {
            assert_eq!(
                map.decide(
                    &crate::keymap::KeyEvent {
                        key,
                        code: Some("Backquote"),
                        modifiers: crate::keymap::Modifiers {
                            control: true,
                            shift,
                            ..Default::default()
                        },
                        repeat: false,
                        composing: false,
                    },
                    crate::keymap::Context {
                        terminal: true,
                        editor: false,
                    },
                    true,
                    now,
                ),
                crate::keymap::Decision::Dispatch(expected.into())
            );
        }
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(Sink));
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let bridge = HostBridge::connect_with_clipboard_ports(
            services,
            Arc::new(move || signal.notify_one()),
            Arc::new(|_| panic!("clipboard write is not allowed")),
            Arc::new(|| panic!("clipboard read is not allowed")),
            None,
        )
        .unwrap();
        let mut previous = snapshot.clone();
        for _ in [false, true] {
            let Some(ShellIntent::NewTerminal { project, pane }) =
                action("new-terminal", &previous)
            else {
                panic!("expected new terminal")
            };
            bridge
                .submit(HostCommand::NewTerminal {
                    project,
                    pane,
                    title: "synthetic new terminal".into(),
                })
                .unwrap();
            tokio::time::timeout(DEADLINE, ready.notified())
                .await
                .unwrap();
            let current = ShellSnapshot::read(&state).await;
            let tab = current.focused_tab().unwrap();
            assert_ne!(tab.id, previous.focused_tab().unwrap().id);
            assert!(!tab.preview && !tab.pinned);
            assert!(
                matches!(&tab.kind, TabKind::Terminal { session_id, cwd } if session_id.is_empty() && cwd.is_none())
            );
            assert_eq!(current.layouts[&background], other);
            assert_eq!(activate(&current), Some(first_editor.id.clone()));
            previous = current;
        }
        tokio::time::timeout(DEADLINE, bridge.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[test]
    fn terminal_tab_gate는_프로젝트없는_새터미널만_소비한다() {
        let context = egui::Context::default();
        let mut views = crate::terminal_surface::Views::default();
        for (shift, expected) in [(false, false), (true, true)] {
            let event = Event::Key {
                key: Key::Backtick,
                physical_key: Some(Key::Backtick),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    shift,
                    ..Default::default()
                },
            };
            let mut actions = Vec::new();
            let mut observed = Vec::new();
            let mut output = context.run_ui(
                egui::RawInput {
                    events: vec![event],
                    ..Default::default()
                },
                |ui| {
                    let events = ui.input_mut(|input| std::mem::take(&mut input.events));
                    for (index, event) in events.iter().enumerate() {
                        observed.push(
                            views
                                .route_keymap(
                                    crate::keymap::Route {
                                        context: ui.ctx(),
                                        event,
                                        index,
                                        scope: Default::default(),
                                        composing: false,
                                        overrides: None,
                                    },
                                    &mut actions,
                                    false,
                                )
                                .unwrap(),
                        );
                    }
                },
            );
            output.textures_delta.clear();
            assert_eq!(observed, [expected]);
            if expected {
                assert_eq!(actions, ["new-terminal"]);
            } else {
                assert!(actions.is_empty());
            }
        }
    }

    #[test]
    fn 명령행_override는_window_route에서_실행경로가_있는_명령만_소비한다() {
        let context = egui::Context::default();
        let mut views = crate::terminal_surface::Views::default();
        let modifiers = if cfg!(target_os = "macos") {
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL | egui::Modifiers::COMMAND
        };
        for (overrides, expected) in [
            (
                r#"[{"actionId":"settings.open","key":"j","mods":["mod"]}]"#,
                Some("settings.open"),
            ),
            (
                r#"[{"actionId":"app.openSettingsFile","key":"j","mods":["mod"]}]"#,
                Some("app.openSettingsFile"),
            ),
            (
                r#"[{"actionId":"sync.uploadNow","key":"j","mods":["mod"]}]"#,
                None,
            ),
            (
                r#"[{"actionId":"window.reload","key":"j","mods":["mod"]}]"#,
                None,
            ),
        ] {
            let event = Event::Key {
                key: Key::J,
                physical_key: Some(Key::J),
                pressed: true,
                repeat: false,
                modifiers,
            };
            let mut actions = Vec::new();
            let mut observed = Vec::new();
            let mut output = context.run_ui(
                egui::RawInput {
                    events: vec![event],
                    ..Default::default()
                },
                |ui| {
                    let events = ui.input_mut(|input| std::mem::take(&mut input.events));
                    for (index, event) in events.iter().enumerate() {
                        observed.push(
                            views
                                .route_keymap(
                                    crate::keymap::Route {
                                        context: ui.ctx(),
                                        event,
                                        index,
                                        scope: Default::default(),
                                        composing: false,
                                        overrides: Some(overrides),
                                    },
                                    &mut actions,
                                    false,
                                )
                                .unwrap(),
                        );
                    }
                },
            );
            output.textures_delta.clear();
            assert_eq!(observed, [expected.is_some()], "{overrides}");
            assert_eq!(
                actions,
                expected.into_iter().collect::<Vec<_>>(),
                "{overrides}"
            );
        }
    }

    #[tokio::test]
    async fn reopen_keymap은_project_stack과_identity_fallback을_runtime로_복원한다() {
        use taide_model::layout::Tab;
        let project = ProjectId::new();
        let background = ProjectId::new();
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-reopen-keymap-{project}")),
        ));
        let mut layout = taide_layout::service::default_layout();
        let pane = layout.focused_pane.clone();
        let make_tab = |index| Tab {
            id: TabId::new(),
            kind: TabKind::Untitled { index },
            title: format!("synthetic reopen {index}"),
            pinned: false,
            preview: false,
            dirty: true,
            view_state: None,
        };
        let first = make_tab(0);
        let last = make_tab(1);
        for tab in [&first, &last] {
            taide_layout::service::open_tab(&mut layout, &pane, tab.clone(), false).unwrap();
            taide_layout::service::close_tab(&mut layout, &tab.id).unwrap();
        }
        layout.closed_tabs.last_mut().unwrap().pane_id = PaneId::new();
        let other = taide_layout::service::default_layout();
        state.layouts.write().extend([
            (project.clone(), layout),
            (background.clone(), other.clone()),
        ]);
        let mut snapshot = ShellSnapshot::read(&state).await;
        assert!(supports("reopen-closed-tab"));
        assert!(action("reopen-closed-tab", &snapshot).is_none());
        let slot = ShellSlotId::new();
        {
            let mut session = state.session.write();
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            });
            session.focused_shell_slot = Some(slot);
        }
        snapshot = ShellSnapshot::read(&state).await;
        let mut map = crate::keymap::Keymap::new().unwrap();
        assert_eq!(
            map.decide(
                &crate::keymap::KeyEvent {
                    key: "T",
                    code: Some("KeyT"),
                    modifiers: crate::keymap::Modifiers {
                        meta: true,
                        shift: true,
                        ..Default::default()
                    },
                    repeat: false,
                    composing: false,
                },
                Default::default(),
                true,
                std::time::Instant::now(),
            ),
            crate::keymap::Decision::Dispatch("reopen-closed-tab".into())
        );
        for expected in [&last, &first] {
            let Some(ShellIntent::Mutate(mutation @ ShellMutation::ReopenClosed(_))) =
                action("reopen-closed-tab", &snapshot)
            else {
                panic!("expected typed reopen")
            };
            let revision = snapshot.layouts[&project].revision;
            taide_native_ui::commands::dispatch(&Sink, &state, mutation)
                .await
                .unwrap();
            snapshot = ShellSnapshot::read(&state).await;
            let tab = snapshot.focused_tab().unwrap();
            assert_eq!(tab.id, expected.id);
            assert_eq!(tab.kind, expected.kind);
            assert_eq!(tab.title, expected.title);
            assert!(!tab.dirty && !tab.preview);
            assert_eq!(snapshot.layouts[&project].focused_pane, pane);
            assert_eq!(snapshot.layouts[&project].revision, revision + 1);
            assert_eq!(snapshot.layouts[&background], other);
        }
        let restored = snapshot.layouts[&project].clone();
        assert!(restored.closed_tabs.is_empty());
        let Some(ShellIntent::Mutate(mutation)) = action("reopen-closed-tab", &snapshot) else {
            panic!("expected empty-stack request")
        };
        taide_native_ui::commands::dispatch(&Sink, &state, mutation)
            .await
            .unwrap();
        assert_eq!(state.layouts.read()[&project], restored);
        let mut missing_active = snapshot.clone();
        let PaneNode::Leaf { active, .. } =
            &mut missing_active.layouts.get_mut(&project).unwrap().root
        else {
            panic!("expected leaf")
        };
        *active = None;
        assert!(matches!(
            action("reopen-closed-tab", &missing_active),
            Some(ShellIntent::Mutate(ShellMutation::ReopenClosed(_)))
        ));
    }

    #[tokio::test]
    async fn group_keymap은_중첩방향_번호_noop와_pinned_이동의_runtime를_보존한다() {
        use taide_model::layout::Tab;
        const RATIO: f32 = 50.0;
        const GROUPS: usize = 4;
        const LAST_POSITION: usize = 9;
        let panes: [_; GROUPS] = std::array::from_fn(|_| PaneId::new());
        let leaf = |id: &PaneId, pinned| {
            let tab = Tab {
                id: TabId::new(),
                kind: TabKind::Untitled { index: 1 },
                title: "synthetic group".into(),
                pinned,
                preview: false,
                dirty: false,
                view_state: None,
            };
            PaneNode::Leaf {
                id: id.clone(),
                active: Some(tab.id.clone()),
                tabs: vec![tab],
            }
        };
        let mut source = leaf(&panes[1], true);
        let PaneNode::Leaf {
            tabs: source_tabs,
            active,
            ..
        } = &mut source
        else {
            panic!("expected source")
        };
        let moved = active.clone().unwrap();
        let PaneNode::Leaf { tabs: extra, .. } = leaf(&panes[1], false) else {
            panic!("expected extra")
        };
        source_tabs.extend(extra);
        let mut target = leaf(&panes[2], true);
        let PaneNode::Leaf {
            tabs: target_tabs, ..
        } = &mut target
        else {
            panic!("expected target")
        };
        let pinned = target_tabs[0].id.clone();
        let PaneNode::Leaf { tabs: extra, .. } = leaf(&panes[2], false) else {
            panic!("expected extra")
        };
        let ordinary = extra[0].id.clone();
        target_tabs.extend(extra);
        let mut layout = taide_layout::service::default_layout();
        layout.focused_pane = panes[1].clone();
        layout.root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            sizes: vec![RATIO, RATIO],
            children: vec![
                PaneNode::Split {
                    id: PaneId::new(),
                    dir: SplitDir::Vertical,
                    sizes: vec![RATIO, RATIO],
                    children: vec![leaf(&panes[0], false), source],
                },
                PaneNode::Split {
                    id: PaneId::new(),
                    dir: SplitDir::Vertical,
                    sizes: vec![RATIO, RATIO],
                    children: vec![target, leaf(&panes[3], false)],
                },
            ],
        };
        let data = std::env::temp_dir().join(format!("taide-group-keymap-{}", ProjectId::new()));
        let state = AppState::new(AppPaths::new(data));
        let project = ProjectId::new();
        let background = ProjectId::new();
        let other = taide_layout::service::default_layout();
        state.layouts.write().extend([
            (project.clone(), layout.clone()),
            (background.clone(), other.clone()),
        ]);
        let slot = ShellSlotId::new();
        {
            let mut session = state.session.write();
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            });
            session.focused_shell_slot = Some(slot);
        }
        let snapshot = ShellSnapshot::read(&state).await;
        let target_of = |id: &str, snapshot: &ShellSnapshot| match action(id, snapshot) {
            Some(ShellIntent::Mutate(ShellMutation::FocusPane(pane))) => Some(pane),
            None => None,
            _ => panic!("expected focus intent"),
        };
        assert_eq!(
            target_of("focus-group-up", &snapshot),
            Some(panes[0].clone())
        );
        assert_eq!(
            target_of("focus-group-right", &snapshot),
            Some(panes[2].clone())
        );
        assert!(target_of("focus-group-left", &snapshot).is_none());
        assert!(target_of("focus-group-down", &snapshot).is_none());
        for position in 1..=LAST_POSITION {
            let id = format!("focus-group-{position}");
            assert!(supports(&id));
            let expected = panes
                .get(position - 1)
                .filter(|pane| **pane != layout.focused_pane)
                .cloned();
            assert_eq!(target_of(&id, &snapshot), expected);
        }
        let mut empty = snapshot.clone();
        let PaneNode::Split { children, .. } = &mut empty.layouts.get_mut(&project).unwrap().root
        else {
            panic!("expected root")
        };
        let PaneNode::Split { children, .. } = &mut children[0] else {
            panic!("expected nested")
        };
        let PaneNode::Leaf { tabs, active, .. } = &mut children[1] else {
            panic!("expected empty group")
        };
        tabs.clear();
        *active = None;
        assert_eq!(
            target_of("focus-group-right", &empty),
            Some(panes[2].clone())
        );
        assert!(action("move-tab-to-group-right", &empty).is_none());
        let mut missing = snapshot.clone();
        missing.layouts.get_mut(&project).unwrap().focused_pane = PaneId::new();
        assert!(target_of("focus-group-right", &missing).is_none());
        assert_eq!(target_of("focus-group-1", &missing), Some(panes[0].clone()));
        let mut map = crate::keymap::Keymap::new().unwrap();
        let event = |key| crate::keymap::KeyEvent {
            key,
            code: None,
            modifiers: crate::keymap::Modifiers {
                meta: true,
                ..Default::default()
            },
            repeat: false,
            composing: false,
        };
        let now = std::time::Instant::now();
        assert_eq!(
            map.decide(&event("k"), Default::default(), true, now),
            crate::keymap::Decision::EnterChord
        );
        assert_eq!(
            map.decide(&event("ArrowRight"), Default::default(), true, now),
            crate::keymap::Decision::ResolveChord("focus-group-right".into())
        );
        let Some(ShellIntent::Mutate(focus)) = action("focus-group-right", &snapshot) else {
            panic!("expected focused neighbor")
        };
        taide_native_ui::commands::dispatch(&Sink, &state, focus)
            .await
            .unwrap();
        let focused = ShellSnapshot::read(&state).await;
        assert_eq!(focused.layouts[&project].focused_pane, panes[2]);
        assert_eq!(
            target_of("focus-group-left", &focused),
            Some(panes[1].clone())
        );
        assert_eq!(
            target_of("focus-group-down", &focused),
            Some(panes[3].clone())
        );
        state.layouts.write().insert(project.clone(), layout);
        let Some(ShellIntent::Mutate(mutation @ ShellMutation::MoveTab { .. })) =
            action("move-tab-to-group-right", &snapshot)
        else {
            panic!("expected move intent")
        };
        taide_native_ui::commands::dispatch(&Sink, &state, mutation)
            .await
            .unwrap();
        let current = ShellSnapshot::read(&state).await;
        let layout = &current.layouts[&project];
        assert_eq!(layout.focused_pane, panes[2]);
        let PaneNode::Leaf { tabs, active, .. } =
            taide_layout::service::find_leaf(&layout.root, &panes[2]).unwrap()
        else {
            panic!("expected destination")
        };
        assert_eq!(
            tabs.iter().map(|tab| tab.id.clone()).collect::<Vec<_>>(),
            [pinned, moved.clone(), ordinary]
        );
        assert_eq!(active.as_ref(), Some(&moved));
        assert_eq!(current.layouts[&background], other);
        if state.paths.data_dir.exists() {
            std::fs::remove_dir_all(&state.paths.data_dir).unwrap();
        }
    }

    #[tokio::test]
    async fn shell_keymap은_실제_editor_gate와_focused_shell_worker를_연결한다() {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-native-keymap-{}", ProjectId::new())),
        ));
        let focused = ProjectId::new();
        let background = ProjectId::new();
        let slot = ShellSlotId::new();
        {
            let mut session = state.session.write();
            session.projects = [focused.clone(), background.clone()]
                .into_iter()
                .map(|id| ProjectRef {
                    id,
                    root: "/synthetic".into(),
                    name: "synthetic keymap".into(),
                    display: Default::default(),
                    root_missing: false,
                })
                .collect();
            session.active_project = Some(focused.clone());
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: focused.clone(),
            });
            session.focused_shell_slot = Some(slot);
        }
        let layout = taide_layout::service::default_layout();
        let original_background = taide_layout::service::default_layout();
        state.layouts.write().insert(focused.clone(), layout);
        state
            .layouts
            .write()
            .insert(background.clone(), original_background.clone());
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(Sink));
        let connection =
            ShellController::connect(state.clone(), &tasks, Arc::new(Sink), Arc::new(|| {}))
                .await
                .unwrap();
        let mut controller = connection.controller;
        state.settings.write().keymap_overrides =
            Some(r#"[{"actionId":"toggle-sidebar","key":"j","mods":[]}]"#.into());
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: UNDO_GROUPS,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), "", "plaintext".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "main".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        let editor = NativeEditor {
            appearance: EditorAppearance {
                font: FontId::monospace(FONT_SIZE),
                line_height: LINE_HEIGHT,
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
        let mut terminal_views = crate::terminal_surface::Views::default();
        let mut actions = Vec::new();
        let key = |key| Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        {
            let mut draw = |events| {
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
                                terminal_views
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
                                            overrides: state
                                                .settings
                                                .read()
                                                .keymap_overrides
                                                .as_deref(),
                                        },
                                        &mut actions,
                                        true,
                                    )
                                    .unwrap()
                            })
                            .unwrap();
                        assert!(output.errors.is_empty());
                        if ui.ctx().current_pass_index() == 0 {
                            ui.ctx()
                                .request_discard("synthetic editor keymap multi-pass");
                        }
                    },
                );
                output.textures_delta.clear();
            };
            draw(vec![key(Key::J), Event::Text("j".into())]);
            draw(vec![key(Key::X), Event::Text("x".into())]);
            draw(vec![
                Event::Ime(ImeEvent::Preedit {
                    text: "漢".into(),
                    active_range_chars: None,
                }),
                key(Key::J),
                Event::Ime(ImeEvent::Commit("漢".into())),
            ]);
        }
        assert_eq!(actions, ["toggle-sidebar"]);
        let modifiers = if cfg!(target_os = "macos") {
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL | egui::Modifiers::COMMAND
        };
        let close = Event::Key {
            key: Key::W,
            physical_key: Some(Key::W),
            pressed: true,
            repeat: false,
            modifiers,
        };
        let mut output = context.run_ui(
            egui::RawInput {
                events: vec![close.clone()],
                ..Default::default()
            },
            |ui| {
                assert!(
                    !terminal_views
                        .route_keymap(
                            crate::keymap::Route {
                                context: ui.ctx(),
                                event: &close,
                                index: 0,
                                scope: Default::default(),
                                composing: false,
                                overrides: None
                            },
                            &mut actions,
                            false
                        )
                        .unwrap()
                );
            },
        );
        output.textures_delta.clear();
        assert_eq!(actions, ["toggle-sidebar"]);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "x漢"
        );
        let before = controller.snapshot();
        let Some(ShellIntent::Mutate(sidebar)) = action(&actions[0], &before) else {
            panic!("sidebar mutation is missing")
        };
        controller.submit(sidebar).unwrap();
        let after = tokio::time::timeout(DEADLINE, async {
            loop {
                let current = controller.changed().await.unwrap();
                if current.layouts[&focused].shell_view.sidebar_collapsed
                    != before.layouts[&focused].shell_view.sidebar_collapsed
                {
                    break current;
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(after.layouts[&background], original_background);
        let first = after.focused_tab().unwrap().id.clone();
        let Some(ShellIntent::Mutate(cycle)) = action("tab-cycle-prev", &after) else {
            panic!("cycle mutation is missing")
        };
        controller.submit(cycle).unwrap();
        let cycled = tokio::time::timeout(DEADLINE, async {
            loop {
                let current = controller.changed().await.unwrap();
                if current.focused_tab().unwrap().id != first {
                    break current;
                }
            }
        })
        .await
        .unwrap();
        let active = cycled.focused_tab().unwrap().id.clone();
        assert!(
            matches!(action("close-tab", &cycled), Some(ShellIntent::RequestCloseTab(tab)) if tab == active)
        );
        let Some(ShellIntent::Mutate(split)) = action("split", &cycled) else {
            panic!("split mutation is missing")
        };
        controller.submit(split).unwrap();
        let split = tokio::time::timeout(DEADLINE, async {
            loop {
                let current = controller.changed().await.unwrap();
                if matches!(current.layouts[&focused].root, PaneNode::Split { .. }) {
                    break current;
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(split.layouts[&background], original_background);
        let mut zen = split.as_ref().clone();
        zen.shell.window_chrome.zen = true;
        assert!(action("toggle-sidebar", &zen).is_none());
        zen.shell.focused = None;
        assert!(action("split", &zen).is_none());
        drop(controller);
        tokio::time::timeout(DEADLINE, connection.worker)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
        drop(services);
        if state.paths.data_dir.exists() {
            std::fs::remove_dir_all(&state.paths.data_dir).unwrap();
        }
    }
}
