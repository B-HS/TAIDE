use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::file::FsChangeKind;
use taide_model::ids::{PaneId, ProjectId};
use taide_model::layout::{PaneNode, ProjectLayout, TabKind};
pub(crate) use taide_native_ui::command_palette::{Action, Appearance, FileIndex, Palette, Scope};

const STALE_AFTER: Duration = Duration::from_secs(60);
const UNOBSERVED_RETENTION: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
pub(crate) struct FileIndexChanges {
    projects: Mutex<HashSet<ProjectId>>,
}

impl FileIndexChanges {
    pub(crate) fn record(&self, event: &AppEvent) {
        let project = match event {
            AppEvent::FsRescanRequired { project_id } => project_id,
            AppEvent::FsChanged { project_id, change } if change.kind != FsChangeKind::Modified => {
                project_id
            }
            _ => return,
        };
        self.projects
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(project.clone());
    }

    pub(crate) fn take(&self) -> HashSet<ProjectId> {
        std::mem::take(
            &mut *self
                .projects
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        )
    }
}

#[derive(Default)]
struct Entry {
    paths: Option<Vec<String>>,
    revision: u64,
    fetched_at: Option<Instant>,
    released_at: Option<Instant>,
    is_invalidated: bool,
    is_fetching: bool,
    has_failed: bool,
}

#[derive(Default)]
pub(crate) struct FileIndexes {
    entries: HashMap<ProjectId, Entry>,
    scoped: Option<ProjectId>,
    observed: Option<ProjectId>,
    revisions: u64,
}

impl FileIndexes {
    pub(crate) fn observe(
        &mut self,
        project: Option<&ProjectId>,
        is_enabled: bool,
        now: Instant,
    ) -> Option<ProjectId> {
        if self.scoped.as_ref() != project {
            if let Some(entry) = self
                .scoped
                .take()
                .and_then(|previous| self.entries.get_mut(&previous))
            {
                entry.released_at = Some(now);
            }
            self.scoped = project.cloned();
            if let Some(entry) = project.and_then(|project| self.entries.get_mut(project)) {
                entry.released_at = None;
            }
        }
        self.entries.retain(|_, entry| {
            entry.is_fetching
                || entry.released_at.is_none_or(|released_at| {
                    now.saturating_duration_since(released_at) < UNOBSERVED_RETENTION
                })
        });
        let observed = project.filter(|_| is_enabled);
        let was_observed = self.observed.as_ref() == observed;
        self.observed = observed.cloned();
        let project = observed?;
        let is_continuing = was_observed && self.entries.contains_key(project);
        let entry = self.entries.entry(project.clone()).or_default();
        let is_stale = entry.paths.is_none()
            || entry.is_invalidated
            || entry
                .fetched_at
                .is_none_or(|fetched_at| now.saturating_duration_since(fetched_at) >= STALE_AFTER);
        let needs_fetch = if is_continuing {
            entry.is_invalidated
        } else {
            is_stale
        };
        if entry.is_fetching || !needs_fetch {
            return None;
        }
        entry.is_fetching = true;
        entry.is_invalidated = false;
        entry.has_failed = false;
        Some(project.clone())
    }

    pub(crate) fn accept(
        &mut self,
        project: &ProjectId,
        result: AppResult<Vec<String>>,
        now: Instant,
    ) {
        let Some(entry) = self.entries.get_mut(project) else {
            return;
        };
        entry.is_fetching = false;
        match result {
            Ok(paths) => {
                self.revisions += 1;
                entry.revision = self.revisions;
                entry.paths = Some(paths);
                entry.fetched_at = Some(now);
                entry.has_failed = false;
            }
            Err(_) => entry.has_failed = true,
        }
    }

    pub(crate) fn invalidate(&mut self, project: &ProjectId) {
        if let Some(entry) = self.entries.get_mut(project) {
            entry.is_invalidated = true;
        }
    }

    pub(crate) fn retain(&mut self, is_open: impl Fn(&ProjectId) -> bool) {
        self.entries.retain(|project, _| is_open(project));
    }

    pub(crate) fn cancel_fetches(&mut self) {
        for entry in self.entries.values_mut() {
            if std::mem::take(&mut entry.is_fetching) {
                entry.is_invalidated = true;
            }
        }
        self.observed = None;
    }

    pub(crate) fn view<'a>(
        &'a self,
        project: Option<&ProjectId>,
        root: Option<&'a str>,
    ) -> FileIndex<'a> {
        let entry = project.and_then(|project| self.entries.get(project));
        FileIndex {
            root,
            paths: entry.and_then(|entry| entry.paths.as_deref()),
            revision: entry.map_or(0, |entry| entry.revision),
            is_pending: project.is_some()
                && entry.is_none_or(|entry| entry.paths.is_none() && !entry.has_failed),
            is_refreshing: entry.is_some_and(|entry| entry.is_fetching),
        }
    }
}

pub(crate) fn file_tab_pane(layout: &ProjectLayout, path: &str) -> Option<PaneId> {
    let holds_file = |leaf: &&PaneNode| {
        matches!(leaf, PaneNode::Leaf { tabs, .. } if tabs.iter().any(|tab| {
            matches!(&tab.kind, TabKind::File { path: candidate } if candidate == path)
        }))
    };
    let leaf = taide_layout::service::find_leaf(&layout.root, &layout.focused_pane)
        .filter(holds_file)
        .or_else(|| {
            taide_layout::service::collect_leaves(&layout.root)
                .into_iter()
                .find(holds_file)
        })?;
    match leaf {
        PaneNode::Leaf { id, .. } => Some(id.clone()),
        PaneNode::Split { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_model::error::AppError;
    use taide_model::file::FsChange;
    use taide_model::ids::TabId;
    use taide_model::layout::{SplitDir, Tab};

    const HALF: f32 = 0.5;
    const ROOT: &str = "/synthetic/project";

    fn paths(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| format!("{ROOT}/{name}")).collect()
    }

    fn changed(project: &ProjectId, kind: FsChangeKind, from_app: bool) -> AppEvent {
        AppEvent::FsChanged {
            project_id: project.clone(),
            change: FsChange {
                kind,
                paths: paths(&["src/main.rs"]),
                from_app,
            },
        }
    }

    fn file_tab(path: &str) -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::File { path: path.into() },
            title: "synthetic palette file".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }
    }

    fn leaf(id: &PaneId, tabs: Vec<Tab>) -> PaneNode {
        PaneNode::Leaf {
            id: id.clone(),
            active: tabs.first().map(|tab| tab.id.clone()),
            tabs,
        }
    }

    #[test]
    fn 파일_목록은_파일_모드로_열릴_때만_조회하고_60초_동안_다시_조회하지_않는다() {
        let project = ProjectId::new();
        let start = Instant::now();
        let mut indexes = FileIndexes::default();
        assert_eq!(indexes.observe(Some(&project), false, start), None);
        assert_eq!(indexes.observe(None, true, start), None);
        let closed = indexes.view(Some(&project), Some(ROOT));
        assert!(closed.is_pending && !closed.is_refreshing && closed.paths.is_none());
        let projectless = indexes.view(None, None);
        assert!(!projectless.is_pending && projectless.paths.is_none());

        assert_eq!(
            indexes.observe(Some(&project), true, start),
            Some(project.clone())
        );
        assert_eq!(indexes.observe(Some(&project), true, start), None);
        let loading = indexes.view(Some(&project), Some(ROOT));
        assert!(loading.is_pending && loading.is_refreshing);

        indexes.accept(&project, Ok(paths(&["a.rs", "b.rs"])), start);
        let loaded = indexes.view(Some(&project), Some(ROOT));
        assert!(!loaded.is_pending && !loaded.is_refreshing);
        assert_eq!(loaded.paths, Some(paths(&["a.rs", "b.rs"]).as_slice()));
        assert_eq!(loaded.root, Some(ROOT));
        let revision = loaded.revision;

        assert_eq!(indexes.observe(Some(&project), false, start), None);
        let fresh = start + STALE_AFTER - Duration::from_secs(1);
        assert_eq!(
            indexes.observe(Some(&project), true, fresh),
            None,
            "신선한 목록은 다시 열어도 조회하지 않는다"
        );
        assert_eq!(indexes.observe(Some(&project), false, fresh), None);
        let stale = start + STALE_AFTER;
        assert_eq!(
            indexes.observe(Some(&project), true, stale),
            Some(project.clone()),
            "60초가 지난 목록은 다시 열 때 조회한다"
        );
        let refreshing = indexes.view(Some(&project), Some(ROOT));
        assert!(!refreshing.is_pending && refreshing.is_refreshing);
        assert_eq!(refreshing.paths.map(<[String]>::len), Some(2));
        indexes.accept(&project, Ok(paths(&["a.rs"])), stale);
        let refreshed = indexes.view(Some(&project), Some(ROOT));
        assert_eq!(refreshed.paths.map(<[String]>::len), Some(1));
        assert!(refreshed.revision > revision);
        assert_eq!(
            indexes.observe(Some(&project), true, stale + STALE_AFTER * 2),
            None,
            "열려 있는 동안에는 시간이 지나도 다시 조회하지 않는다"
        );
    }

    #[test]
    fn 무효화된_목록은_열려_있으면_즉시_닫혀_있으면_다음에_열_때_다시_조회한다() {
        let project = ProjectId::new();
        let now = Instant::now();
        let mut indexes = FileIndexes::default();
        indexes.invalidate(&project);
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            Some(project.clone())
        );
        indexes.accept(&project, Ok(paths(&["a.rs"])), now);

        indexes.invalidate(&project);
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            Some(project.clone())
        );
        indexes.invalidate(&project);
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            None,
            "조회 중에는 겹쳐 조회하지 않는다"
        );
        indexes.accept(&project, Ok(paths(&["a.rs", "b.rs"])), now);
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            Some(project.clone()),
            "조회 중에 들어온 무효화는 응답 뒤에 한 번 더 조회한다"
        );
        indexes.accept(&project, Ok(paths(&["a.rs", "b.rs", "c.rs"])), now);
        assert_eq!(indexes.observe(Some(&project), true, now), None);

        assert_eq!(indexes.observe(Some(&project), false, now), None);
        indexes.invalidate(&project);
        assert_eq!(indexes.observe(Some(&project), false, now), None);
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            Some(project.clone())
        );
        let stale = indexes.view(Some(&project), Some(ROOT));
        assert!(stale.is_refreshing && !stale.is_pending);
        assert_eq!(stale.paths.map(<[String]>::len), Some(3));
    }

    #[test]
    fn 조회_실패는_빈_결과로_남고_다시_열_때_재시도한다() {
        let project = ProjectId::new();
        let now = Instant::now();
        let mut indexes = FileIndexes::default();
        assert!(indexes.observe(Some(&project), true, now).is_some());
        indexes.accept(
            &project,
            Err(AppError::Internal("synthetic walk failure".into())),
            now,
        );
        let failed = indexes.view(Some(&project), Some(ROOT));
        assert!(!failed.is_pending && !failed.is_refreshing && failed.paths.is_none());
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            None,
            "실패한 조회를 매 프레임 반복하지 않는다"
        );
        assert_eq!(indexes.observe(Some(&project), false, now), None);
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            Some(project.clone())
        );
        assert!(indexes.view(Some(&project), Some(ROOT)).is_pending);
        indexes.accept(&project, Ok(paths(&["a.rs"])), now);
        assert!(indexes.observe(Some(&project), false, now).is_none());
        indexes.invalidate(&project);
        assert!(indexes.observe(Some(&project), true, now).is_some());
        indexes.accept(
            &project,
            Err(AppError::Internal("synthetic refresh failure".into())),
            now,
        );
        let kept = indexes.view(Some(&project), Some(ROOT));
        assert_eq!(
            kept.paths.map(<[String]>::len),
            Some(1),
            "갱신 실패는 이전 목록을 유지한다"
        );

        assert!(indexes.observe(Some(&project), false, now).is_none());
        indexes.invalidate(&project);
        assert!(indexes.observe(Some(&project), true, now).is_some());
        indexes.cancel_fetches();
        assert!(!indexes.view(Some(&project), Some(ROOT)).is_refreshing);
        assert_eq!(
            indexes.observe(Some(&project), true, now),
            Some(project.clone()),
            "호스트가 끊겨 취소된 조회는 다시 요청한다"
        );
    }

    #[test]
    fn 프로젝트가_바뀌면_그_프로젝트의_목록만_보이고_관찰이_끝난_목록은_10분_뒤_버린다() {
        let first = ProjectId::new();
        let second = ProjectId::new();
        let start = Instant::now();
        let mut indexes = FileIndexes::default();
        assert!(indexes.observe(Some(&first), true, start).is_some());
        indexes.accept(&first, Ok(paths(&["a.rs"])), start);
        assert_eq!(
            indexes.observe(Some(&second), true, start),
            Some(second.clone())
        );
        let switched = indexes.view(Some(&second), Some(ROOT));
        assert!(switched.is_pending && switched.paths.is_none());
        indexes.accept(&second, Ok(paths(&["b.rs", "c.rs"])), start);
        assert_ne!(
            indexes.view(Some(&first), Some(ROOT)).revision,
            indexes.view(Some(&second), Some(ROOT)).revision
        );

        let retained = start + UNOBSERVED_RETENTION - Duration::from_secs(1);
        assert!(indexes.observe(Some(&second), false, retained).is_none());
        assert!(indexes.view(Some(&first), Some(ROOT)).paths.is_some());
        let expired = start + UNOBSERVED_RETENTION;
        assert!(indexes.observe(Some(&second), false, expired).is_none());
        assert!(indexes.view(Some(&first), Some(ROOT)).paths.is_none());
        assert!(indexes.view(Some(&second), Some(ROOT)).paths.is_some());

        indexes.retain(|project| project != &second);
        assert!(indexes.view(Some(&second), Some(ROOT)).paths.is_none());
        indexes.accept(&second, Ok(paths(&["late.rs"])), expired);
        assert!(
            indexes.view(Some(&second), Some(ROOT)).paths.is_none(),
            "닫힌 프로젝트의 늦은 응답은 버린다"
        );
    }

    #[test]
    fn 파일_집합을_바꾸는_변경과_rescan만_목록을_무효화한다() {
        let project = ProjectId::new();
        let other = ProjectId::new();
        let changes = FileIndexChanges::default();
        changes.record(&changed(&project, FsChangeKind::Modified, false));
        changes.record(&changed(&project, FsChangeKind::Modified, true));
        changes.record(&AppEvent::ProjectClosed {
            project_id: project.clone(),
        });
        assert!(changes.take().is_empty());
        for kind in [
            FsChangeKind::Created,
            FsChangeKind::Removed,
            FsChangeKind::Renamed,
        ] {
            changes.record(&changed(&project, kind, true));
            assert_eq!(changes.take(), HashSet::from([project.clone()]));
        }
        changes.record(&changed(&project, FsChangeKind::Created, false));
        changes.record(&AppEvent::FsRescanRequired {
            project_id: other.clone(),
        });
        assert_eq!(changes.take(), HashSet::from([project, other]));
        assert!(changes.take().is_empty());
    }

    #[tokio::test]
    async fn 팔레트_키는_창_route에서_소비되어_진입_질의로_열리고_고른_파일_경로를_돌려준다() {
        use crate::command_registry::PaletteEntry;
        use eframe::egui::{self, Event, Key, Modifiers};
        use taide_model::ids::ShellSlotId;
        use taide_model::paths::AppPaths;
        use taide_model::project::{ProjectRef, ShellSlotTree};
        use taide_native_ui::commands::ShellIntent;
        use taide_native_ui::shell::WindowScope;
        use taide_native_ui::snapshot::ShellSnapshot;
        use taide_runtime::AppState;

        const SCREEN: [f32; 2] = [1000.0, 800.0];
        const FRAME_STEP: f64 = 0.1;
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-command-palette-route-{project}")),
        ));
        let layout = taide_layout::service::default_layout();
        state
            .layouts
            .write()
            .insert(project.clone(), layout.clone());
        let slot = ShellSlotId::new();
        {
            let mut session = state.session.write();
            session.projects.push(ProjectRef {
                id: project.clone(),
                root: ROOT.into(),
                name: "synthetic palette".into(),
                display: Default::default(),
                root_missing: false,
            });
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            });
            session.focused_shell_slot = Some(slot);
        }
        let snapshot = ShellSnapshot::read(&state).await;
        let commands = crate::command_dispatch::context(&snapshot, &WindowScope::Main, None);
        let root = snapshot
            .project(&project)
            .map(|project| project.root.as_str());
        assert_eq!(root, Some(ROOT));
        let theme = taide_runtime::theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
        let locale = taide_model::locale::ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            messages: serde_json::from_str(include_str!(
                "../../../crates/taide-locale/resources/locales/en.json"
            ))
            .unwrap(),
            warnings: Vec::new(),
        };
        let context = egui::Context::default();
        let mut views = crate::terminal_surface::Views::default();
        views.set_command_context(commands.clone());
        let mut palette = Palette::new(&theme, "en-US", cfg!(target_os = "macos")).unwrap();
        let mut indexes = FileIndexes::default();
        let mut time = 0.0;
        let mut frame = |palette: &mut Palette,
                         indexes: &mut FileIndexes,
                         views: &mut crate::terminal_surface::Views,
                         events: Vec<Event>| {
            time += FRAME_STEP;
            let mut routed = Vec::new();
            let mut output = None;
            let mut drawing = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(SCREEN[0], SCREEN[1]),
                    )),
                    time: Some(time),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ui| {
                    let mut actions = Vec::new();
                    views
                        .route_window_keys(
                            ui.ctx(),
                            Default::default(),
                            false,
                            None,
                            &mut actions,
                            true,
                        )
                        .unwrap();
                    for action in &actions {
                        if let Some(ShellIntent::OpenPalette(entry)) =
                            crate::command_dispatch::intent(action, &commands, &snapshot)
                        {
                            palette.open(ui.ctx(), entry);
                        }
                    }
                    routed = actions;
                    let now = Instant::now();
                    if let Some(stale) =
                        indexes.observe(Some(&project), palette.observes_files(), now)
                    {
                        indexes.accept(&stale, Ok(paths(&["src/main.rs", "README.md"])), now);
                    }
                    output = Some(
                        palette
                            .show(
                                ui.ctx(),
                                Scope {
                                    locale: &locale,
                                    commands: &commands,
                                    keymap_overrides: None,
                                    files: indexes.view(Some(&project), root),
                                    active_file: None,
                                    symbols: Default::default(),
                                },
                                true,
                            )
                            .unwrap(),
                    );
                },
            );
            drawing.textures_delta.clear();
            (routed, output.unwrap())
        };
        let primary = if cfg!(target_os = "macos") {
            Modifiers::MAC_CMD | Modifiers::COMMAND
        } else {
            Modifiers::CTRL | Modifiers::COMMAND
        };
        let shortcut = |key: Key, shift: bool| Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: Modifiers { shift, ..primary },
        };

        for (key, shift, action, entry, query) in [
            (Key::P, true, "command-palette", PaletteEntry::Commands, ">"),
            (
                Key::T,
                false,
                "workspace-symbol",
                PaletteEntry::WorkspaceSymbols,
                "#",
            ),
            (Key::P, false, "quick-open", PaletteEntry::Files, ""),
        ] {
            let (routed, output) = frame(
                &mut palette,
                &mut indexes,
                &mut views,
                vec![shortcut(key, shift)],
            );
            assert_eq!(routed, [action], "{entry:?}");
            assert_eq!(output.action, None);
            assert!(palette.is_open());
            assert_eq!(palette.inspection().query, query, "{entry:?}");
        }
        frame(&mut palette, &mut indexes, &mut views, Vec::new());
        let listed = palette.inspection().clone();
        assert!(listed.has_input_focus);
        assert_eq!(
            listed
                .rows
                .iter()
                .map(|row| row.key.as_str())
                .collect::<Vec<_>>(),
            [
                "/synthetic/project/src/main.rs",
                "/synthetic/project/README.md"
            ]
        );
        frame(
            &mut palette,
            &mut indexes,
            &mut views,
            vec![Event::Text("readme".into())],
        );
        let (routed, output) = frame(
            &mut palette,
            &mut indexes,
            &mut views,
            vec![Event::Key {
                key: Key::Enter,
                physical_key: Some(Key::Enter),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(routed.is_empty());
        let Some(Action::OpenFile(path)) = output.action else {
            panic!("expected the selected file to open");
        };
        assert_eq!(path, "/synthetic/project/README.md");
        assert_eq!(file_tab_pane(&layout, &path), None);
        assert!(!palette.is_open());
        if state.paths.data_dir.exists() {
            std::fs::remove_dir_all(&state.paths.data_dir).unwrap();
        }
    }

    #[test]
    fn 이미_열린_파일은_포커스_pane_다음_트리_순서의_pane에서_연다() {
        let left = PaneId::new();
        let right = PaneId::new();
        let both = format!("{ROOT}/both.rs");
        let only_left = format!("{ROOT}/left.rs");
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![
                leaf(&left, vec![file_tab(&both), file_tab(&only_left)]),
                leaf(&right, vec![file_tab(&both)]),
            ],
            sizes: vec![HALF, HALF],
        };
        layout.focused_pane = right.clone();
        assert_eq!(file_tab_pane(&layout, &both), Some(right.clone()));
        assert_eq!(file_tab_pane(&layout, &only_left), Some(left.clone()));
        assert_eq!(file_tab_pane(&layout, &format!("{ROOT}/new.rs")), None);
        layout.focused_pane = PaneId::new();
        assert_eq!(file_tab_pane(&layout, &both), Some(left));
    }
}
