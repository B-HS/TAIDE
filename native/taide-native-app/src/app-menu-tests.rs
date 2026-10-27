use super::*;

use std::time::Duration;

use taide_model::paths::AppPaths;
use taide_model::project::ProjectRef;

const DEADLINE: Duration = Duration::from_secs(5);
const EXTRA_PROJECTS: usize = 5;
const KEPT_DRAFTS: u32 = 2;

fn locale(language: &str) -> ResolvedLocale {
    let messages = match language {
        "ko" => include_str!("../../../crates/taide-locale/resources/locales/ko.json"),
        _ => include_str!("../../../crates/taide-locale/resources/locales/en.json"),
    };
    ResolvedLocale {
        id: language.into(),
        name: language.into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(messages).unwrap(),
    }
}

fn project(name: &str, last_opened_at: f64) -> Project {
    Project {
        id: ProjectId::new(),
        root: format!("/synthetic/{name}"),
        name: name.into(),
        capabilities: Vec::new(),
        root_missing: false,
        last_opened_at,
        display: Default::default(),
    }
}

fn item(row: &Row) -> (&str, &str, bool) {
    match row {
        Row::Item {
            id, text, enabled, ..
        } => (id.as_str(), text.as_str(), *enabled),
        row => panic!("expected an item row, got {row:?}"),
    }
}

#[derive(Clone, Default)]
struct Recording(Arc<Mutex<Vec<Model>>>);

impl Bar for Recording {
    fn apply(&mut self, model: &Model) {
        self.0.lock().unwrap().push(model.clone());
    }
}

impl Recording {
    fn last(&self) -> Option<Model> {
        self.0.lock().unwrap().last().cloned()
    }

    fn count(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

struct Drawing<'a> {
    bar: &'a Recording,
    locale: &'a ResolvedLocale,
    app: &'a AppState,
    tasks: &'a TaskSupervisor,
    context: &'a egui::Context,
}

impl Drawing<'_> {
    async fn until(&self, state: &mut State, projects: &[Project]) {
        let expected = Model::new(self.locale, projects);
        tokio::time::timeout(DEADLINE, async {
            while self.bar.last().as_ref() != Some(&expected) {
                assert!(
                    state
                        .tick(self.locale, self.app, self.tasks, self.context)
                        .is_empty()
                );
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}

#[test]
fn 메뉴_id는_각자의_행동으로_해석된다() {
    let project = ProjectId::new();
    assert_eq!(action(QUIT_ID), Action::Quit);
    assert_eq!(action(CLEAR_RECENT_ID), Action::ClearRecent);
    assert_eq!(
        action(&recent_id(&project)),
        Action::OpenRecent(project.clone())
    );
    for (id, edit) in [
        (UNDO_ID, Edit::Undo),
        (REDO_ID, Edit::Redo),
        (CUT_ID, Edit::Cut),
        (COPY_ID, Edit::Copy),
        (PASTE_ID, Edit::Paste),
        (SELECT_ALL_ID, Edit::SelectAll),
    ] {
        assert_eq!(action(id), Action::Edit(edit));
    }
    for ignored in [FILE_ID, OPEN_RECENT_ID, NO_RECENT_ID, RECENT_ID_PREFIX, ""] {
        assert_eq!(action(ignored), Action::Ignored);
    }
    assert_eq!(recent_project(&recent_id(&project)), Some(project));
}

#[test]
fn 최근_항목은_상한까지_원래_순서로_남고_표시_라벨과_폴더_누락을_따른다() {
    let mut projects: Vec<Project> = (0..crate::welcome::RECENT_MENU_LIMIT + EXTRA_PROJECTS)
        .map(|index| project(&format!("p{index}"), index as f64))
        .collect();
    projects[0].display.label = Some(" TA ".into());
    projects[1].display.label = Some("   ".into());
    projects[2].root_missing = true;
    let model = Model::new(&locale("en"), &projects);
    let (rows, tail) = model.recent.split_at(crate::welcome::RECENT_MENU_LIMIT);
    let expected: Vec<(String, String, bool)> = projects
        .iter()
        .take(crate::welcome::RECENT_MENU_LIMIT)
        .enumerate()
        .map(|(index, project)| {
            let text = if index == 0 {
                "TA"
            } else {
                project.name.as_str()
            };
            (recent_id(&project.id), text.to_owned(), index != 2)
        })
        .collect();
    let actual: Vec<(String, String, bool)> = rows
        .iter()
        .map(|row| {
            let (id, text, enabled) = item(row);
            (id.to_owned(), text.to_owned(), enabled)
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(tail[0], Row::Separator);
    assert_eq!(item(&tail[1]), (CLEAR_RECENT_ID, "Clear Recent", true));
    assert_eq!(tail.len(), 2);
}

#[test]
fn 최근_항목이_없으면_비활성_안내와_비활성_지우기가_선다() {
    let model = Model::new(&locale("ko"), &[]);
    assert_eq!(model.file, "파일");
    assert_eq!(model.open_recent, "최근 항목 열기");
    assert_eq!(model.recent.len(), 3);
    assert_eq!(
        item(&model.recent[0]),
        (NO_RECENT_ID, "최근 프로젝트 없음", false)
    );
    assert_eq!(model.recent[1], Row::Separator);
    assert_eq!(
        item(&model.recent[2]),
        (CLEAR_RECENT_ID, "최근 항목 지우기", false)
    );
}

#[test]
fn 메뉴바는_원본_순서의_앱_파일_편집_창_메뉴를_만든다() {
    let model = Model::new(&locale("en"), &[project("alpha", 1.0)]);
    let bar = model.bar();
    assert_eq!(
        bar.iter()
            .map(|submenu| (submenu.id, submenu.title.as_str()))
            .collect::<Vec<_>>(),
        [
            (None, "TAIDE"),
            (Some(FILE_ID), "File"),
            (None, "Edit"),
            (None, "Window"),
        ]
    );
    assert_eq!(
        bar[0].rows,
        vec![
            Row::Standard(Standard::About),
            Row::Separator,
            Row::Standard(Standard::Services),
            Row::Separator,
            Row::Standard(Standard::Hide),
            Row::Standard(Standard::HideOthers),
            Row::Standard(Standard::ShowAll),
            Row::Separator,
            Row::Item {
                id: QUIT_ID.into(),
                text: "Quit".into(),
                enabled: true,
                shortcut: Some(Shortcut::CommandQ),
            },
        ]
    );
    assert_eq!(
        bar[1].rows,
        vec![Row::Submenu {
            id: OPEN_RECENT_ID,
            title: "Open Recent".into(),
            rows: model.recent.clone(),
        }]
    );
    assert_eq!(
        bar[2]
            .rows
            .iter()
            .map(|row| match row {
                Row::Separator => None,
                row => Some(item(row)),
            })
            .collect::<Vec<_>>(),
        [
            Some((UNDO_ID, "Undo", true)),
            Some((REDO_ID, "Redo", true)),
            None,
            Some((CUT_ID, "Cut", true)),
            Some((COPY_ID, "Copy", true)),
            Some((PASTE_ID, "Paste", true)),
            Some((SELECT_ALL_ID, "Select All", true)),
        ]
    );
    assert!(bar[2].rows.iter().all(|row| !matches!(
        row,
        Row::Item {
            shortcut: Some(_),
            ..
        }
    )));
    assert_eq!(
        bar[3].rows,
        vec![
            Row::Standard(Standard::Minimize),
            Row::Standard(Standard::Maximize),
            Row::Separator,
            Row::Standard(Standard::Fullscreen),
        ]
    );
    assert_eq!(literal_text("R&D"), "R&&D");
}

#[test]
fn 편집_항목은_요청_명령이나_원래_단축키_입력으로_포커스_입력에_전달된다() {
    let command = command_modifiers();
    assert!(command.command);
    assert_eq!(Edit::Undo.input(), Input::Chord(egui::Key::Z, command));
    assert_eq!(
        Edit::Redo.input(),
        Input::Chord(egui::Key::Z, command | egui::Modifiers::SHIFT)
    );
    assert_eq!(Edit::SelectAll.input(), Input::Chord(egui::Key::A, command));
    assert_eq!(
        Edit::Cut.input(),
        Input::Request(egui::ViewportCommand::RequestCut)
    );
    assert_eq!(
        Edit::Copy.input(),
        Input::Request(egui::ViewportCommand::RequestCopy)
    );
    assert_eq!(
        Edit::Paste.input(),
        Input::Request(egui::ViewportCommand::RequestPaste)
    );
    let child = egui::ViewportId::from_hash_of("synthetic child");
    let mut state = State::new(Arc::default());
    state.queue_chord(egui::ViewportId::ROOT, egui::Key::A, command);
    let mut events = Vec::new();
    state.inject(child, &mut events);
    assert!(events.is_empty());
    state.inject(egui::ViewportId::ROOT, &mut events);
    assert_eq!(
        events,
        [true, false].map(|pressed| egui::Event::Key {
            key: egui::Key::A,
            physical_key: Some(egui::Key::A),
            pressed,
            repeat: false,
            modifiers: command,
        })
    );
    state.inject(egui::ViewportId::ROOT, &mut events);
    assert_eq!(events.len(), 2);
    state.queue_chord(egui::ViewportId::ROOT, egui::Key::Z, command);
    state.finish_child_passes();
    assert!(state.chord.is_some());
    state.queue_chord(child, egui::Key::Z, command);
    state.finish_child_passes();
    assert!(state.chord.is_none());
}

#[test]
fn 최근_목록_변경_이벤트만_다시_읽기를_요청하고_초안_보존_수를_한번씩_넘긴다() {
    let changes = Changes::default();
    let project = ProjectId::new();
    changes.record(&AppEvent::ProjectGroupsChanged { groups: Vec::new() });
    assert_eq!(changes.revision(), 0);
    changes.record(&AppEvent::ProjectListChanged {
        projects: Vec::<ProjectRef>::new(),
    });
    changes.record(&AppEvent::ProjectActivated {
        project_id: Some(project),
    });
    assert_eq!(changes.revision(), 2);
    changes.record(&AppEvent::ProjectRecentCleared {
        removed: 1,
        skipped_with_drafts: 0,
    });
    changes.record(&AppEvent::ProjectRecentCleared {
        removed: 0,
        skipped_with_drafts: KEPT_DRAFTS,
    });
    assert_eq!(changes.take_kept_drafts(), [KEPT_DRAFTS]);
    assert!(changes.take_kept_drafts().is_empty());
}

#[tokio::test]
async fn 연결된_메뉴바는_최근_목록을_그리고_변경에_다시_그리며_클릭을_넘긴다() {
    let directory = std::env::temp_dir().join(format!("taide-app-menu-{}", ProjectId::new()));
    let paths = AppPaths::new(directory.join("data"));
    let write = |name: &str, last_opened_at: f64| {
        let root = directory.join(name);
        std::fs::create_dir_all(&root).unwrap();
        let project = Project {
            root: root.to_str().unwrap().into(),
            ..project(name, last_opened_at)
        };
        let file = paths.project_file(&project.id);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, serde_json::to_vec(&project).unwrap()).unwrap();
        project
    };
    let older = write("older", 1.0);
    let app = AppState::new(AppPaths::new(directory.join("data")));
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let context = egui::Context::default();
    let changes = Arc::new(Changes::default());
    let mut state = State::new(changes.clone());
    let korean = locale("ko");
    let locale = locale("en");
    assert!(state.tick(&locale, &app, &tasks, &context).is_empty());
    let bar = Recording::default();
    let (sender, receiver) = mpsc::channel();
    state.connect(Box::new(bar.clone()), receiver);
    let fixture = Drawing {
        bar: &bar,
        locale: &locale,
        app: &app,
        tasks: &tasks,
        context: &context,
    };
    fixture
        .until(&mut state, std::slice::from_ref(&older))
        .await;
    let applied = bar.count();
    for _ in 0..EXTRA_PROJECTS {
        state.tick(&locale, &app, &tasks, &context);
        tokio::task::yield_now().await;
    }
    assert_eq!(bar.count(), applied, "an unchanged list is not redrawn");
    state.tick(&korean, &app, &tasks, &context);
    assert_eq!(
        bar.last(),
        Some(Model::new(&korean, std::slice::from_ref(&older))),
        "a locale change redraws the titles and rows"
    );
    assert_eq!(bar.count(), applied + 1);
    let newer = write("newer", 2.0);
    changes.record(&AppEvent::ProjectListChanged {
        projects: Vec::new(),
    });
    fixture
        .until(&mut state, &[newer.clone(), older.clone()])
        .await;
    sender.send(recent_id(&older.id)).unwrap();
    sender.send(QUIT_ID.into()).unwrap();
    assert_eq!(
        state.tick(&locale, &app, &tasks, &context),
        [Action::OpenRecent(older.id.clone()), Action::Quit]
    );
    tokio::time::timeout(DEADLINE, async {
        while tasks.tracked_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
