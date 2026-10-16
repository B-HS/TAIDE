use super::*;

use std::time::Duration;

use taide_model::paths::AppPaths;

const DEADLINE: Duration = Duration::from_secs(5);
const PROJECTS: usize = RECENT_DISPLAY_LIMIT + 2;

fn locale() -> ResolvedLocale {
    ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    }
}

fn project(index: usize, root: &std::path::Path, missing: bool) -> Project {
    let root = if missing {
        root.join(format!("missing-{index}"))
    } else {
        root.to_path_buf()
    };
    Project {
        id: ProjectId::new(),
        root: root.to_string_lossy().into_owned(),
        name: format!("project-{index}"),
        capabilities: Vec::new(),
        root_missing: false,
        last_opened_at: index as f64,
        display: Default::default(),
    }
}

#[test]
fn welcome_단축키는_원본_순서와_재지정_해제와_번역을_반영한다() {
    let mut state = State::default();
    let rows = state.shortcuts(None, &locale(), true).unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (row.description.as_str(), row.label.as_str()))
            .collect::<Vec<_>>(),
        [
            ("Quick Open File", "⌘P"),
            ("Command Palette", "⇧⌘P"),
            ("Search", "⇧⌘F"),
            ("Toggle Terminal", "⌃`"),
            ("Toggle Sidebar", "⌘B"),
            ("Save", "⌘S"),
        ]
    );
    let overrides = r#"[{"actionId":"save","key":"","mods":[]},{"actionId":"quick-open","key":"o","mods":["mod","alt"]}]"#;
    let rows = state.shortcuts(Some(overrides), &locale(), false).unwrap();
    assert_eq!(rows[0].label, "Ctrl+Alt+O");
    assert_eq!(rows[5].description, "Save");
    assert_eq!(rows[5].label, "");
    let cached = state.shortcuts(Some(overrides), &locale(), false).unwrap();
    assert!(Arc::ptr_eq(&rows, &cached));
}

#[tokio::test]
async fn welcome_최근목록은_자기를_제외하고_상한과_root누락과_실패를_구분한다() {
    let directory = std::env::temp_dir().join(format!("taide-welcome-{}", ProjectId::new()));
    let root = directory.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let paths = AppPaths::new(directory.join("data"));
    let projects = (0..PROJECTS)
        .map(|index| project(index, &root, index == PROJECTS - 2))
        .collect::<Vec<_>>();
    for project in &projects {
        let file = paths.project_file(&project.id);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, serde_json::to_vec(project).unwrap()).unwrap();
    }
    let app = AppState::new(paths);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let context = eframe::egui::Context::default();
    let snapshot = Arc::new(ShellSnapshot::read(&app).await);
    let mut state = State::default();
    assert_eq!(state.content(None, &[]).recent, Some(Vec::new()));
    state.tick(&snapshot, &app, &tasks, &context);
    tokio::time::timeout(DEADLINE, async {
        while state.recent.is_none() {
            tokio::task::yield_now().await;
            state.tick(&snapshot, &app, &tasks, &context);
        }
    })
    .await
    .unwrap();
    let newest = &projects[PROJECTS - 1];
    let recent = state.content(Some(&newest.id), &[]).recent.unwrap();
    assert_eq!(recent.len(), RECENT_DISPLAY_LIMIT);
    assert!(recent.iter().all(|row| row.id != newest.id));
    assert_eq!(recent[0].name, format!("project-{}", PROJECTS - 2));
    assert!(recent[0].root_missing);
    assert!(recent[1..].iter().all(|row| !row.root_missing));
    assert_eq!(state.content(None, &[]).recent.unwrap()[0].id, newest.id);
    state.tick(&snapshot, &app, &tasks, &context);
    assert!(state.pending.is_none(), "the same snapshot does not reload");
    state.recent = Some(Err(()));
    assert_eq!(state.content(None, &[]).recent, None);
    state.clear();
    tokio::time::timeout(DEADLINE, async {
        while tasks.tracked_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
