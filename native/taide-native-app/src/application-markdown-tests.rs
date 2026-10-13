use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, key, paint, text_contains, wait,
};
use super::*;

const ORIGINAL: &str = "# Markdown heading\n\n**live preview** and `code`\n\n<ul><li type='A' value='27'>first item</li></ul>\n\n<h2>HTML preview heading</h2>\n\n<ol start='-1'><li>first ordered</li><li>second ordered</li></ol>\n\n<ol reversed type='a'><li type='A'>reverse first</li><li type='i' value='-4'>reset</li><li type='I'>reverse last</li></ol>\n\n<table><caption>Spanning caption</caption><tfoot><tr><td colspan='3'>footer data</td></tr></tfoot><thead><tr><th colspan='3'>group head</th></tr></thead><tbody><tr><td colspan='2'>longer content<img src='./diagram.png' width='80' height='40' alt='table image'></td><td rowspan='2'>tall data</td></tr><tr><td>first</td><td>second</td></tr></tbody></table>\n\n<img src='./diagram.png' width='800' height='400' alt='owned image'>\n";
const UPDATED: &str = "# Updated heading\n\nchanged body";
const IMAGE_SIZE: [u32; 2] = [2, 1];
const IMAGE_PIXELS: &[u8] = &[255, 0, 0, 255, 0, 255, 0, 255];
const EXCESSIVE_NESTING: usize = 256;
const CHANGED_EDITOR_FONT_SIZE: u32 = 30;
const PREVIEW_HEADING_SIZE: f32 = 24.0;
const CHANGED_THEME: &str = "vscode-light-modern";
const SCROLL_PARAGRAPHS: usize = 80;
const SCROLL_DELTA: f32 = 180.0;
const SETTLE_FRAMES: usize = 16;
const INVALID_UTF8: u8 = 0xff;

fn toggle_preview(application: &mut NativeApplication, context: &egui::Context, pane: &PaneId) {
    let position = application
        .editor_markdown
        .geometry(context.viewport_id(), pane)
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
    paint(application, context, Vec::new());
}

fn scroll_at(application: &mut NativeApplication, context: &egui::Context, position: egui::Pos2) {
    paint(
        application,
        context,
        vec![egui::Event::PointerMoved(position)],
    );
    paint(
        application,
        context,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -SCROLL_DELTA),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for _ in 0..SETTLE_FRAMES {
        paint(application, context, Vec::new());
    }
}

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
fn 실제_markdown_표_셀은_문단_목록_코드와_중첩_표를_분리해_표시한다() {
    const CONTENT: &str = "<table><caption><p><code>first caption</code><code>second code</code><code><strong>styled</strong> plain</code></p><p>second caption</p><p><a href='https://example.com/caption'><img src='./cell.png' width='80' height='40'></a></p><ol start='9883'><li>caption list item</li></ol><ul><li><input disabled type='checkbox'>caption task words wrap across several lines and use the entire list item width</li></ul><pre><code>caption code\nlonger caption code</code></pre></caption><tr><td><p>first paragraph</p><p>second paragraph</p><ol><li>list item</li></ol><pre><code>let cell = 1;\nnext line</code></pre><blockquote><p>linked paragraph<a href='https://example.com'>safe link</a><img src='./cell.png' width='80' height='40'></p></blockquote><table><tr><td>nested table</td></tr></table></td><td>neighbor</td></tr></table>";
    const CELL_IMAGE_SIZE: egui::Vec2 = egui::vec2(80.0, 40.0);
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        "original rust",
    );
    let path = PathBuf::from(&fixture.path).with_file_name("cell-blocks.md");
    let image = path.with_file_name("cell.png");
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
    let image_bytes = std::fs::read(&image).unwrap();
    std::fs::write(&path, CONTENT).unwrap();
    let path = path.to_str().unwrap().to_owned();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: None,
        path: path.clone(),
        preview: false,
    }));
    wait(
        application,
        context,
        "table blocks live view",
        |application| {
            application.files.get(&path).is_some_and(|document| {
                application
                    .store
                    .views()
                    .for_document(document.id)
                    .next()
                    .is_some()
            })
        },
    );
    let document = application.files[&path].id;
    let key = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap()
        .key
        .clone();
    paint(application, context, Vec::new());
    toggle_preview(application, context, &key.pane);
    wait(
        application,
        context,
        "table block image visible",
        |application| {
            application
                .editor_markdown
                .image_rects(context.viewport_id(), &key.pane)
                .len()
                == 2
        },
    );
    let output = paint(application, context, Vec::new());
    for label in [
        "first caption",
        "second code",
        "styled",
        " plain",
        "second caption",
        "9883.",
        "caption list item",
        "caption task words wrap across several lines and use the entire list item width",
        "caption code\nlonger caption code",
        "first paragraph",
        "second paragraph",
        "1.",
        "list item",
        "let cell = 1;\nnext line",
        "neighbor",
    ] {
        assert!(
            preview_text_contains(application, context, &key.pane, &output, label),
            "missing table block {label}"
        );
    }
    let images = application
        .editor_markdown
        .image_rects(context.viewport_id(), &key.pane);
    assert_eq!(images.len(), 2);
    assert_eq!(images[1].size(), CELL_IMAGE_SIZE);
    assert_eq!(images[0].size(), CELL_IMAGE_SIZE);
    let position = application
        .editor_markdown
        .geometry(context.viewport_id(), &key.pane)
        .2
        .unwrap()
        .center();
    scroll_at(application, context, position);
    let output = paint(application, context, Vec::new());
    assert!(preview_text_contains(
        application,
        context,
        &key.pane,
        &output,
        "nested table"
    ));
    assert_eq!(std::fs::read(&image).unwrap(), image_bytes);
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        CONTENT
    );
    assert!(application.submit(HostCommand::CloseTab {
        tab: key.tab,
        discard: false
    }));
    wait(application, context, "table blocks closed", |application| {
        !application.files.contains_key(&path)
    });
    assert_eq!(std::fs::read_to_string(&path).unwrap(), CONTENT);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
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
    for number in ["AA.", "-1.", "0.", "C.", "-4.", "-5."] {
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
    assert!(
        application
            .editor_markdown
            .image_rects(context.viewport_id(), &pane)
            .len()
            > 1
    );
    assert!(image.width() <= preview.width());
    assert!(image.width() > 0.0);
    assert!(
        (image.width() / image.height() - IMAGE_SIZE[0] as f32 / IMAGE_SIZE[1] as f32).abs()
            < f32::EPSILON
    );
    assert_eq!(body_view(application, &path, &tab), view);
    let preview_scroll = application
        .editor_markdown
        .scroll(context.viewport_id(), &pane)
        .1;
    scroll_at(application, context, preview_scroll.center());
    let output = paint(application, context, Vec::new());
    for label in ["Spanning caption", "group head", "footer data", "tall data"] {
        assert!(
            preview_text_contains(application, context, &pane, &output, label),
            "preview label: {label}"
        );
    }
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

#[test]
fn 실제_markdown_두_뷰는_토글과_본문_미리보기_scroll을_분리하고_mirror_undo를_공유한다() {
    let text = (0..SCROLL_PARAGRAPHS)
        .map(|index| format!("Paragraph {index}\n\n"))
        .collect::<String>();
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        "original rust",
    );
    let path = PathBuf::from(&fixture.path).with_file_name("scroll.md");
    std::fs::write(&path, &text).unwrap();
    let path = path.to_str().unwrap().to_owned();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: None,
        path: path.clone(),
        preview: false,
    }));
    wait(application, context, "markdown main view", |application| {
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
    let main = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap()
        .clone();
    let main_body = crate::editor_context_menu::body(context, main.id).unwrap();
    assert!(application.submit(HostCommand::OpenFileToSide(
        taide_model::layout::OpenTabInSplitRequest {
            project_id: fixture.project.clone(),
            target_pane: main.key.pane.clone(),
            edge: taide_model::layout::DropEdge::Right,
            kind: TabKind::File { path: path.clone() },
            title: "scroll.md".into(),
            preview: false,
        },
    )));
    wait(
        application,
        context,
        "markdown mirror view",
        |application| application.store.views().for_document(document).count() > 1,
    );
    let mirror = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.id != main.id)
        .unwrap()
        .clone();
    toggle_preview(application, context, &main.key.pane);
    assert!(
        application
            .editor_markdown
            .geometry(context.viewport_id(), &mirror.key.pane)
            .2
            .is_none()
    );
    toggle_preview(application, context, &mirror.key.pane);
    assert_eq!(
        crate::editor_context_menu::body(context, main.id).unwrap(),
        main_body
    );
    context.memory_mut(|memory| memory.request_focus(main_body));
    paint(
        application,
        context,
        key(egui::Key::ArrowRight, egui::Modifiers::NONE),
    );
    assert_eq!(
        application
            .store
            .views()
            .get(main.id)
            .unwrap()
            .selection
            .selections[0]
            .head,
        1
    );
    assert_eq!(
        application
            .store
            .views()
            .get(mirror.id)
            .unwrap()
            .selection
            .selections[0]
            .head,
        0
    );
    let main_before = application.store.views().get(main.id).unwrap().clone();
    let mirror_before = application.store.views().get(mirror.id).unwrap().clone();
    let preview = application
        .editor_markdown
        .scroll(context.viewport_id(), &main.key.pane)
        .1;
    scroll_at(application, context, preview.center());
    let preview_offset = application
        .editor_markdown
        .scroll(context.viewport_id(), &main.key.pane)
        .0;
    assert!(preview_offset.y > 0.0);
    assert_eq!(
        application
            .editor_markdown
            .scroll(context.viewport_id(), &mirror.key.pane)
            .0,
        egui::Vec2::ZERO
    );
    assert_eq!(
        application.store.views().get(main.id).unwrap().scroll,
        main_before.scroll
    );
    assert_eq!(
        application.store.views().get(mirror.id).unwrap().scroll,
        mirror_before.scroll
    );
    let editor = application
        .editor_markdown
        .geometry(context.viewport_id(), &main.key.pane)
        .1;
    scroll_at(application, context, editor.center());
    assert!(application.store.views().get(main.id).unwrap().scroll.y > main_before.scroll.y);
    assert_eq!(
        application
            .editor_markdown
            .scroll(context.viewport_id(), &main.key.pane)
            .0,
        preview_offset
    );
    assert_eq!(
        application.store.views().get(mirror.id).unwrap().scroll,
        mirror_before.scroll
    );
    let mirror_body = crate::editor_context_menu::body(context, mirror.id).unwrap();
    context.memory_mut(|memory| memory.request_focus(mirror_body));
    paint(
        application,
        context,
        vec![egui::Event::Paste("# Mirror update\n\n".into())],
    );
    paint(application, context, Vec::new());
    for pane in [&main.key.pane, &mirror.key.pane] {
        application
            .editor_markdown
            .expire_update(context.viewport_id(), pane);
    }
    let output = paint(application, context, Vec::new());
    assert!(preview_text_contains(
        application,
        context,
        &mirror.key.pane,
        &output,
        "Mirror update"
    ));
    assert!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
            .starts_with("# Mirror update")
    );
    assert_eq!(
        application
            .store
            .views()
            .get(main.id)
            .unwrap()
            .selection
            .selections[0]
            .head,
        "# Mirror update\n\n".len() + 1
    );
    context.memory_mut(|memory| memory.request_focus(mirror_body));
    paint(
        application,
        context,
        key(
            egui::Key::Z,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    paint(application, context, Vec::new());
    for pane in [&main.key.pane, &mirror.key.pane] {
        application
            .editor_markdown
            .expire_update(context.viewport_id(), pane);
    }
    paint(application, context, Vec::new());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        text
    );
    toggle_preview(application, context, &main.key.pane);
    assert!(
        application
            .editor_markdown
            .geometry(context.viewport_id(), &main.key.pane)
            .2
            .is_none()
    );
    assert!(
        application
            .editor_markdown
            .geometry(context.viewport_id(), &mirror.key.pane)
            .2
            .is_some()
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    application.request_tab_close(mirror.key.tab.clone());
    if let Some(pending) = &mut application.pending_tab_close {
        pending.automatic_choice = Some(crate::close_dialog::CloseChoice::Discard);
    }
    wait(
        application,
        context,
        "markdown mirror closed",
        |application| application.store.views().get(mirror.id).is_none(),
    );
    assert_eq!(
        application.store.views().get(main.id).unwrap().document,
        document
    );
    assert_eq!(application.files[&path].id, document);
    application.request_tab_close(main.key.tab.clone());
    if let Some(pending) = &mut application.pending_tab_close {
        pending.automatic_choice = Some(crate::close_dialog::CloseChoice::Discard);
    }
    wait(
        application,
        context,
        "markdown main closed",
        |application| !application.files.contains_key(&path),
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_markdown_lossy_readonly는_미리보기_scroll과_선택을_허용하고_입력을_거절한다() {
    let text = (0..SCROLL_PARAGRAPHS)
        .map(|index| format!("Readonly paragraph {index}\n\n"))
        .collect::<String>();
    let bytes = [text.as_bytes(), &[INVALID_UTF8]].concat();
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        "original rust",
    );
    let path = PathBuf::from(&fixture.path).with_file_name("readonly.md");
    std::fs::write(&path, &bytes).unwrap();
    let path = path.to_str().unwrap().to_owned();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: None,
        path: path.clone(),
        preview: false,
    }));
    wait(
        application,
        context,
        "readonly markdown view",
        |application| {
            application.files.get(&path).is_some_and(|document| {
                application
                    .store
                    .views()
                    .for_document(document.id)
                    .next()
                    .is_some()
            })
        },
    );
    let document = application.files[&path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap()
        .clone();
    let before = application.store.documents().snapshot(document).unwrap();
    assert!(before.metadata.read_only && before.metadata.lossy);
    toggle_preview(application, context, &view.key.pane);
    let output = paint(application, context, Vec::new());
    assert!(preview_text_contains(
        application,
        context,
        &view.key.pane,
        &output,
        "Readonly paragraph 0"
    ));
    let body = crate::editor_context_menu::body(context, view.id).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::ArrowRight, egui::Modifiers::NONE),
    );
    assert_eq!(
        application
            .store
            .views()
            .get(view.id)
            .unwrap()
            .selection
            .selections[0]
            .head,
        1
    );
    paint(
        application,
        context,
        vec![
            egui::Event::Paste("changed".into()),
            egui::Event::Text("changed".into()),
        ],
    );
    let preview = application
        .editor_markdown
        .scroll(context.viewport_id(), &view.key.pane)
        .1;
    let editor_before = application
        .store
        .views()
        .get(view.id)
        .unwrap()
        .scroll
        .clone();
    scroll_at(application, context, preview.center());
    assert!(
        application
            .editor_markdown
            .scroll(context.viewport_id(), &view.key.pane)
            .0
            .y
            > 0.0
    );
    assert_eq!(
        application.store.views().get(view.id).unwrap().scroll,
        editor_before
    );
    let after = application.store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.rope.to_string(), before.rope.to_string());
    assert_eq!(after.dirty, before.dirty);
    assert!(after.metadata.read_only && after.metadata.lossy);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(application.submit(HostCommand::CloseTab {
        tab: view.key.tab.clone(),
        discard: false
    }));
    wait(
        application,
        context,
        "readonly markdown closed",
        |application| !application.files.contains_key(&path),
    );
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
