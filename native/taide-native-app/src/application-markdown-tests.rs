use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, key, paint, text_contains, wait,
};
use super::*;

const ORIGINAL: &str = "# Markdown heading\n\n**live preview** and `code`\n\n- first item\n";
const UPDATED: &str = "# Updated heading\n\nchanged body";

#[test]
fn 실제_markdown_토글은_편집기_식별자와_입력_undo_최신_본문을_보존한다() {
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        "original rust",
    );
    let path = PathBuf::from(&fixture.path).with_file_name("preview.md");
    std::fs::write(&path, ORIGINAL).unwrap();
    let path = path.to_str().unwrap().to_owned();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: None,
        path: path.clone(),
        preview: false,
    }));
    wait(application, context, "markdown live view", |application| {
        application.files.get(&path).is_some_and(|document| {
            application
                .store
                .views()
                .for_document(document.id)
                .next()
                .is_some()
        })
    });
    let document = application.files[&path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap();
    let tab = view.key.tab.clone();
    let pane = view.key.pane.clone();
    let view = view.id;
    let body = crate::editor_context_menu::body(context, view).unwrap();
    let before = application
        .store
        .views()
        .get(view)
        .unwrap()
        .selection
        .clone();
    paint(application, context, Vec::new());
    assert!(
        application
            .editor_markdown
            .geometry(context.viewport_id(), &pane)
            .0
            .is_positive()
    );
    let position = application
        .editor_markdown
        .geometry(context.viewport_id(), &pane)
        .0
        .center();
    for pressed in [true, false] {
        paint(
            application,
            context,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let output = paint(application, context, Vec::new());
    assert!(
        application
            .editor_markdown
            .geometry(context.viewport_id(), &pane)
            .2
            .is_some()
    );
    assert!(text_contains(&output, "Markdown heading"));
    assert_eq!(body_view(application, &path, &tab), view);
    assert_eq!(
        crate::editor_context_menu::body(context, view).unwrap(),
        body
    );
    assert_eq!(
        application.store.views().get(view).unwrap().selection,
        before
    );
    let snapshot = application.store.documents().snapshot(document).unwrap();
    let end = snapshot.rope.len_bytes();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: 0,
                    head: end,
                }],
            },
            taide_native_editor::view::ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        vec![egui::Event::Paste(UPDATED.into())],
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        UPDATED
    );
    application
        .editor_markdown
        .expire_update(context.viewport_id(), &pane);
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, "Updated heading"));
    assert!(!text_contains(&output, "Markdown heading"));
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(
            egui::Key::Z,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        ORIGINAL
    );
    application
        .editor_markdown
        .expire_update(context.viewport_id(), &pane);
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, "Markdown heading"));
    assert!(!text_contains(&output, "Updated heading"));
    let other = PathBuf::from(&path).with_file_name("other.md");
    std::fs::write(&other, "# Other heading").unwrap();
    let other = other.to_str().unwrap().to_owned();
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: Some(pane.clone()),
        path: other.clone(),
        preview: false,
    }));
    wait(
        application,
        context,
        "other markdown live view",
        |application| {
            application.files.get(&other).is_some_and(|document| {
                application
                    .store
                    .views()
                    .for_document(document.id)
                    .next()
                    .is_some()
            })
        },
    );
    let other_tab = application
        .store
        .views()
        .for_document(application.files[&other].id)
        .next()
        .unwrap()
        .key
        .tab
        .clone();
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, "Other heading"));
    assert!(!text_contains(&output, "Markdown heading"));
    assert!(
        application
            .editor_markdown
            .geometry(context.viewport_id(), &pane)
            .2
            .is_some()
    );
    assert!(application.submit(HostCommand::CloseTab {
        tab: other_tab,
        discard: false
    }));
    wait(
        application,
        context,
        "other markdown closed",
        |application| !application.files.contains_key(&other),
    );
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, "Markdown heading"));
    application.request_tab_close(tab.clone());
    application
        .pending_tab_close
        .as_mut()
        .unwrap()
        .automatic_choice = Some(crate::close_dialog::CloseChoice::Discard);
    wait(
        application,
        context,
        "preview source closed",
        |application| !application.files.contains_key(&path),
    );
    assert!(
        application
            .editor_markdown
            .geometry(context.viewport_id(), &pane)
            .2
            .is_none()
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), ORIGINAL);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
