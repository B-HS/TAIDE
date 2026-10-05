use std::collections::{HashMap, VecDeque};

use taide_model::{
    ids::TabId,
    layout::{Tab, TabKind},
};
use taide_native_editor::document::DocumentId;

use crate::close_dialog::CloseChoice;

pub(crate) struct Prepared {
    pub tab: Tab,
    pub discard: bool,
    pub approved: Option<(DocumentId, u64)>,
}

pub(crate) enum Task {
    Confirm(Tab),
    Prepare(Tab, CloseChoice),
    Close(Prepared),
}

enum Stage {
    Confirm,
    Prepare(CloseChoice),
    Close,
}

pub(crate) struct Batch {
    tabs: VecDeque<Tab>,
    dirty: VecDeque<Tab>,
    titles: Vec<String>,
    prepared: HashMap<TabId, Prepared>,
    preparing: Option<TabId>,
    stage: Stage,
    pub failure: Option<String>,
}

impl Batch {
    pub fn new(tabs: Vec<Tab>) -> Option<Self> {
        let tabs = tabs
            .into_iter()
            .filter(|tab| !tab.pinned)
            .collect::<VecDeque<_>>();
        if tabs.is_empty() {
            return None;
        }
        let dirty = tabs
            .iter()
            .filter(|tab| {
                tab.dirty && matches!(tab.kind, TabKind::File { .. } | TabKind::Untitled { .. })
            })
            .cloned()
            .collect::<VecDeque<_>>();
        let titles = dirty.iter().map(|tab| tab.title.clone()).collect();
        let stage = if dirty.is_empty() {
            Stage::Close
        } else {
            Stage::Confirm
        };
        Some(Self {
            tabs,
            dirty,
            titles,
            prepared: HashMap::new(),
            preparing: None,
            stage,
            failure: None,
        })
    }

    pub fn titles(&self) -> &[String] {
        &self.titles
    }

    pub fn is_preparing(&self) -> bool {
        matches!(self.stage, Stage::Prepare(_))
    }

    pub fn choose(&mut self, choice: CloseChoice) {
        self.stage = Stage::Prepare(choice);
    }

    pub fn prepared(&mut self, result: Prepared) {
        if let Some(original) = self.preparing.take() {
            self.prepared.insert(original, result);
        }
    }

    pub fn next(&mut self) -> Option<Task> {
        match self.stage {
            Stage::Confirm => return self.dirty.front().cloned().map(Task::Confirm),
            Stage::Prepare(choice) => {
                if self.preparing.is_some() {
                    return None;
                }
                if let Some(tab) = self.dirty.pop_front() {
                    self.preparing = Some(tab.id.clone());
                    return Some(Task::Prepare(tab, choice));
                }
                self.stage = Stage::Close;
            }
            Stage::Close => (),
        }
        let tab = self.tabs.pop_front()?;
        Some(Task::Close(self.prepared.remove(&tab.id).unwrap_or(
            Prepared {
                tab,
                discard: false,
                approved: None,
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{HostBridge, HostCommand, HostReply};
    use std::{path::PathBuf, sync::Arc, time::Duration};
    use taide_model::{
        app_event::AppEvent,
        ids::{ProjectId, ShellSlotId},
        layout::PaneNode,
        paths::AppPaths,
        project::ShellSlotTree,
    };
    use taide_native_ui::{commands::ShellIntent, snapshot::ShellSnapshot};
    use taide_runtime::{AppState, EventSink, TaskSupervisor};
    use tokio::sync::Notify;

    const DEADLINE: Duration = Duration::from_secs(3);
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
    fn tab(title: &str, dirty: bool, pinned: bool) -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::Untitled { index: 1 },
            title: title.into(),
            dirty,
            pinned,
            preview: false,
            view_state: None,
        }
    }
    async fn reply(bridge: &mut HostBridge, ready: &Notify) -> HostReply {
        tokio::time::timeout(DEADLINE, async {
            loop {
                if let Some(result) = bridge.poll() {
                    return result;
                }
                ready.notified().await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn close_all은_한번확인_전체준비뒤_순차host와_취소_변환_고정탭을_보존한다() {
        let directory =
            Directory(std::env::temp_dir().join(format!("taide-close-all-{}", ProjectId::new())));
        std::fs::create_dir_all(&directory.0).unwrap();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        let project = ProjectId::new();
        let slot = ShellSlotId::new();
        {
            let mut session = state.session.write();
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            });
            session.focused_shell_slot = Some(slot);
        }
        let first = tab("first dirty", true, false);
        let clean = tab("clean", false, false);
        let pinned = tab("pinned dirty", true, true);
        let last = tab("last dirty", true, false);
        let tabs = vec![first.clone(), clean.clone(), pinned.clone(), last.clone()];
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Leaf {
            id: layout.focused_pane.clone(),
            active: Some(first.id.clone()),
            tabs: tabs.clone(),
        };
        state
            .layouts
            .write()
            .insert(project.clone(), layout.clone());
        let snapshot = ShellSnapshot::read(&state).await;
        let Some(ShellIntent::RequestCloseTabs(ids)) =
            crate::shell_keymap::action("close-all-tabs", &snapshot)
        else {
            panic!("expected batch intent")
        };
        assert_eq!(ids, [first.id.clone(), clean.id.clone(), last.id.clone()]);
        let mut cancelled = Batch::new(tabs.clone()).unwrap();
        assert_eq!(cancelled.titles(), ["first dirty", "last dirty"]);
        assert!(matches!(cancelled.next(), Some(Task::Confirm(current)) if current.id == first.id));
        drop(cancelled);
        assert_eq!(state.layouts.read()[&project], layout);
        let mut failed_save = Batch::new(tabs.clone()).unwrap();
        failed_save.choose(CloseChoice::Save);
        let Some(Task::Prepare(current, CloseChoice::Save)) = failed_save.next() else {
            panic!("expected first save")
        };
        failed_save.prepared(Prepared {
            tab: current,
            discard: false,
            approved: None,
        });
        assert!(
            matches!(failed_save.next(), Some(Task::Prepare(current, CloseChoice::Save)) if current.id == last.id)
        );
        assert!(failed_save.next().is_none());
        drop(failed_save);
        assert_eq!(state.layouts.read()[&project], layout);
        let mut converted = Batch::new(vec![first.clone(), last.clone()]).unwrap();
        converted.choose(CloseChoice::Save);
        assert!(matches!(
            converted.next(),
            Some(Task::Prepare(_, CloseChoice::Save))
        ));
        let replacement = tab("converted existing file", false, false);
        converted.prepared(Prepared {
            tab: replacement.clone(),
            discard: false,
            approved: None,
        });
        let Some(Task::Prepare(current, CloseChoice::Save)) = converted.next() else {
            panic!("expected second save")
        };
        converted.prepared(Prepared {
            tab: current,
            discard: false,
            approved: None,
        });
        assert!(
            matches!(converted.next(), Some(Task::Close(current)) if current.tab.id == replacement.id)
        );
        assert!(
            matches!(converted.next(), Some(Task::Close(current)) if current.tab.id == last.id)
        );
        assert!(converted.next().is_none());
        let mut batch = Batch::new(tabs).unwrap();
        batch.choose(CloseChoice::Discard);
        for expected in [&first, &last] {
            let Some(Task::Prepare(current, CloseChoice::Discard)) = batch.next() else {
                panic!("expected discard preparation")
            };
            assert_eq!(current.id, expected.id);
            batch.prepared(Prepared {
                tab: current,
                discard: true,
                approved: None,
            });
        }
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(Sink));
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge =
            HostBridge::connect(services, Arc::new(move || signal.notify_one())).unwrap();
        let mut closed = Vec::new();
        while let Some(task) = batch.next() {
            let Task::Close(current) = task else {
                panic!("all preparation must precede close")
            };
            let id = current.tab.id;
            if id == clean.id {
                bridge
                    .submit(HostCommand::CloseTab {
                        tab: id.clone(),
                        discard: false,
                    })
                    .unwrap();
                assert!(matches!(
                    reply(&mut bridge, &ready).await,
                    HostReply::Closed { result: Ok(_), .. }
                ));
            }
            bridge
                .submit(HostCommand::CloseTab {
                    tab: id.clone(),
                    discard: current.discard,
                })
                .unwrap();
            let HostReply::Closed { tab, result } = reply(&mut bridge, &ready).await else {
                panic!("expected close result")
            };
            assert_eq!(tab, id);
            match result {
                Ok(closed) => assert!(!closed.tab.dirty),
                Err(taide_model::error::AppError::NotFound(_)) if id == clean.id => (),
                Err(error) => panic!("unexpected close failure: {error}"),
            }
            closed.push(id);
        }
        assert_eq!(closed, [first.id, clean.id, last.id]);
        let remaining = state.layouts.read()[&project].clone();
        assert_eq!(crate::tabs::tabs_in(&remaining.root), [&pinned]);
        assert!(Batch::new(vec![pinned]).is_none());
        bridge.disconnect().await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }
}
