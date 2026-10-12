use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, key, paint, text_contains, wait,
};
use super::*;

const ORIGINAL: &str = "# Markdown heading\n\n**live preview** and `code`\n\n- first item\n\n<h2>HTML preview heading</h2>\n\n<ol start='-1'><li>first ordered</li><li>second ordered</li></ol>\n\n<table><tr><th>short</th><th>header two</th></tr><tr><td>longer content</td><td>first<br>second</td></tr></table>\n\n<img src='./diagram.png' width='800' height='400' alt='owned image'>\n";
const UPDATED: &str = "# Updated heading\n\nchanged body";
const IMAGE_SIZE: [u32; 2] = [2, 1];
const IMAGE_PIXELS: &[u8] = &[255, 0, 0, 255, 0, 255, 0, 255];
const EXCESSIVE_NESTING: usize = 256;
const CHANGED_EDITOR_FONT_SIZE: u32 = 30;
const PREVIEW_HEADING_SIZE: f32 = 24.0;
const CHANGED_THEME: &str = "vscode-light-modern";

fn preview_text_contains(
    application: &NativeApplication,
    context: &egui::Context,
    pane: &PaneId,
    output: &egui::FullOutput,
    label: &str,
) -> bool {
    let Some(rect) = application
        .editor_markdown
        .geometry(context.viewport_id(), pane)
        .2
    else {
        return false;
    };
    output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.pos.x >= rect.left() && shape.clip_rect.intersects(rect) && text.galley.job.text == label))
}

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
    let image = path.with_file_name("diagram.png");
    let mut png = png::Encoder::new(
        std::fs::File::create(&image).unwrap(),
        IMAGE_SIZE[0],
        IMAGE_SIZE[1],
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(IMAGE_PIXELS)
        .unwrap();
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
    {
        let mut settings = application.services.state.settings.write();
        settings.editor_font_size = CHANGED_EDITOR_FONT_SIZE;
        settings.theme_id = CHANGED_THEME.into();
    }
    application
        .presentation_changes
        .record(&AppEvent::ThemeChanged {
            theme_id: CHANGED_THEME.into(),
        });
    wait(
        application,
        context,
        "markdown theme and editor font changed",
        |application| {
            application.resolved_theme.id == CHANGED_THEME
                && application.editor.appearance.font.size == CHANGED_EDITOR_FONT_SIZE as f32
        },
    );
    assert_eq!(
        application.editor_markdown_colors,
        taide_native_ui::presentation::editor_markdown_colors(&application.resolved_theme).unwrap()
    );
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
    let preview_rect = application
        .editor_markdown
        .geometry(context.viewport_id(), &pane)
        .2
        .unwrap();
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.pos.x >= preview_rect.left() && shape.clip_rect.intersects(preview_rect) && text.galley.job.text == "Markdown heading" && text.galley.job.sections.iter().all(|section| section.format.font_id.size == PREVIEW_HEADING_SIZE && section.format.color == application.editor_markdown_colors.foreground))));
    assert!(preview_text_contains(
        application,
        context,
        &pane,
        &output,
        "HTML preview heading"
    ));
    for number in ["-1.", "0."] {
        assert!(preview_text_contains(
            application,
            context,
            &pane,
            &output,
            number
        ));
    }
    wait(
        application,
        context,
        "relative image loaded",
        |application| {
            !application
                .editor_markdown
                .image_rects(context.viewport_id(), &pane)
                .is_empty()
        },
    );
    let preview = application
        .editor_markdown
        .geometry(context.viewport_id(), &pane)
        .2
        .unwrap();
    let image = application
        .editor_markdown
        .image_rects(context.viewport_id(), &pane)[0];
    assert!(image.width() <= preview.width());
    assert!(image.width() > 0.0);
    assert!(
        (image.width() / image.height() - IMAGE_SIZE[0] as f32 / IMAGE_SIZE[1] as f32).abs()
            < f32::EPSILON
    );
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
    let excessive = format!(
        "{}safe{}",
        "<div>".repeat(EXCESSIVE_NESTING),
        "</div>".repeat(EXCESSIVE_NESTING)
    );
    paint(
        application,
        context,
        vec![egui::Event::Paste(excessive.clone())],
    );
    application
        .editor_markdown
        .expire_update(context.viewport_id(), &pane);
    let output = paint(application, context, Vec::new());
    assert!(preview_text_contains(
        application,
        context,
        &pane,
        &output,
        &presentation::message(&application.locale, "editor.markdownPreviewError", &[])
    ));
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        excessive
    );
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(
            egui::Key::Z,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    application
        .editor_markdown
        .expire_update(context.viewport_id(), &pane);
    let output = paint(application, context, Vec::new());
    assert!(preview_text_contains(
        application,
        context,
        &pane,
        &output,
        "HTML preview heading"
    ));
    assert!(!preview_text_contains(
        application,
        context,
        &pane,
        &output,
        &presentation::message(&application.locale, "editor.markdownPreviewError", &[])
    ));
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
