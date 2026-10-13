use super::inlay_hints_tests::{
    body_view, fixture_with_emmet_peek, fixture_with_language, key, paint, text_contains, wait,
};
use super::*;

#[test]
fn 실제_emmet의_다른파일_html_peek는_source만_수락하고_owner_디스크_닫힘을_보존한다() {
    let mut fixture = fixture_with_emmet_peek();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let owner = body_view(application, &fixture.path, &fixture.tab);
    let owner_document = application.store.views().get(owner).unwrap().document;
    let owner_snapshot = application
        .store
        .documents()
        .snapshot(owner_document)
        .unwrap();
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Request {
            kind: taide_native_editor::symbol_locations::Kind::Definition,
            mode: taide_native_editor::symbol_locations::Mode::Peek,
        }),
    ));
    wait(
        application,
        context,
        "Emmet HTML peek open",
        |application| {
            application
                .editor_locations
                .current(owner)
                .is_some_and(|session| session.preview.is_some())
        },
    );
    let source = application
        .editor_locations
        .current(owner)
        .unwrap()
        .preview
        .unwrap();
    let document = application.store.views().get(source).unwrap().document;
    assert_ne!(document, owner_document);
    let snapshot = application.store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.metadata.language_id, "html");
    assert_eq!(
        snapshot.key,
        taide_native_editor::document::DocumentKey::File(std::path::PathBuf::from(
            &fixture.peek_path
        ))
    );
    let source_state = application.store.views().get(source).unwrap().clone();
    let byte = snapshot.rope.len_bytes();
    application
        .store
        .set_view_state(
            source,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: byte,
                    head: byte,
                }],
            },
            source_state.scroll,
            source_state.folds,
        )
        .unwrap();
    let body = crate::editor_context_menu::body(context, source).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    application
        .editor_completion
        .queue_command(source, taide_native_editor::completion::Command::Trigger);
    wait(
        application,
        context,
        "Emmet HTML peek candidate",
        |application| {
            application
                .editor_completion
                .candidates(&application.store, source)
                .is_some_and(|items| {
                    items.iter().any(|candidate| {
                        candidate.item.detail.as_deref() == Some("Emmet abbreviation")
                    })
                })
        },
    );
    let request = application
        .editor_completion
        .request(&application.store, source)
        .unwrap()
        .clone();
    assert_eq!(request.owner, owner);
    assert_eq!(request.snapshot.id, document);
    paint(
        application,
        context,
        key(egui::Key::Tab, egui::Modifiers::NONE),
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "<div class=\"foo\"></div>"
    );
    let unchanged = application
        .store
        .documents()
        .snapshot(owner_document)
        .unwrap();
    assert_eq!(unchanged.revision, owner_snapshot.revision);
    assert_eq!(unchanged.rope, owner_snapshot.rope);
    assert_eq!(
        std::fs::read_to_string(&fixture.peek_path).unwrap(),
        "div.foo"
    );
    assert_eq!(
        std::fs::read_to_string(&fixture.path).unwrap(),
        "fn owner() {}"
    );
    assert!(application.store.undo(document).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "div.foo"
    );
    application
        .store
        .set_view_state(
            source,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: byte,
                    head: byte,
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    application
        .editor_completion
        .queue_command(source, taide_native_editor::completion::Command::Trigger);
    wait(
        application,
        context,
        "Emmet peek reopened candidate",
        |application| {
            application
                .editor_completion
                .request(&application.store, source)
                .is_some()
        },
    );
    let pending = application
        .editor_completion
        .request(&application.store, source)
        .unwrap()
        .clone();
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Close),
    ));
    wait(
        application,
        context,
        "Emmet peek cancelled",
        |application| {
            application.editor_locations.current(owner).is_none() && pending.is_cancelled()
        },
    );
    assert!(
        application
            .editor_completion
            .request(&application.store, source)
            .is_none()
    );
    eframe::App::on_exit(application);
    application.runtime.block_on(async {
        application.services.tasks.shutdown().await;
    });
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_emmet는_html파일_공급_표시_수락_undo와_설정변경을_연결한다() {
    actual_emmet(
        "div.foo>span",
        "<div class=\"foo\"><span></span></div>",
        "html",
        "html",
    );
}

#[test]
fn 실제_emmet는_css와_scss파일의_축약식을_공급_수락하고_되돌린다() {
    actual_emmet(
        ".box {\n    m10-20",
        ".box {\n    margin: 10px 20px;",
        "css",
        "css",
    );
    actual_emmet(
        ".box {\n    m10",
        ".box {\n    margin: 10px;",
        "scss",
        "scss",
    );
}

#[test]
fn 실제_emmet는_heex파일의_html축약식을_공급_수락하고_되돌린다() {
    actual_emmet("div", "<div></div>", "heex", "heex");
}

#[test]
fn 실제_emmet는_jsx파일의_classname축약식을_공급_수락하고_되돌린다() {
    actual_emmet(
        "div.foo",
        "<div className=\"foo\"></div>",
        "jsx",
        "javascriptreact",
    );
}

#[test]
fn 실제_emmet는_tsx파일의_classname축약식을_공급_수락하고_되돌린다() {
    actual_emmet(
        "div.foo",
        "<div className=\"foo\"></div>",
        "tsx",
        "typescriptreact",
    );
}

fn actual_emmet(original: &str, expected: &str, extension: &str, language: &str) {
    let mut fixture = fixture_with_language(original, extension);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .metadata
            .language_id,
        language
    );
    let state = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: original.len(),
                    head: original.len(),
                }],
            },
            state.scroll,
            state.folds,
        )
        .unwrap();
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Completion(taide_native_editor::completion::Command::Trigger),
    ));
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(application, context, Vec::new());
    assert!(
        application
            .editor_completion
            .request(&application.store, view)
            .is_some(),
        "request absent; selection={:?}; pending={:?}; settings={}; surface={:?}",
        application.store.views().get(view).unwrap().selection,
        application.document_edits,
        application.services.state.settings.read().emmet_enabled,
        application.open_with.surface(&fixture.path)
    );
    wait(application, context, "Emmet candidate", |application| {
        application
            .editor_completion
            .candidates(&application.store, view)
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|candidate| candidate.item.detail.as_deref() == Some("Emmet abbreviation"))
            })
    });
    let snapshot = application.store.documents().snapshot(document).unwrap();
    let tokens = taide_native_editor::language_configuration::LineSyntax::accurate_tokens(
        &crate::editor_syntax::SyntaxLease::new(&mut application.editor_syntax, document),
        &snapshot,
        snapshot.rope.byte_to_line(original.len()),
    );
    assert!(
        tokens.as_ref().is_some_and(|tokens| tokens
            .iter()
            .any(|token| token.kind == taide_native_editor::syntax::TokenKind::Other)),
        "tokens={tokens:?}; candidates={:?}",
        application
            .editor_completion
            .candidates(&application.store, view)
    );
    let output = paint(application, context, Vec::new());
    assert!(text_contains(
        &output,
        original.lines().last().unwrap().trim_start()
    ));
    paint(
        application,
        context,
        key(egui::Key::Tab, egui::Modifiers::NONE),
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    assert!(application.store.undo(document).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        original
    );
    let state = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: original.len(),
                    head: original.len(),
                }],
            },
            state.scroll,
            state.folds,
        )
        .unwrap();
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Completion(taide_native_editor::completion::Command::Trigger),
    ));
    context.memory_mut(|memory| memory.request_focus(body));
    wait(application, context, "Emmet enabled again", |application| {
        application
            .editor_completion
            .candidates(&application.store, view)
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|candidate| candidate.item.detail.as_deref() == Some("Emmet abbreviation"))
            })
    });
    application.services.state.settings.write().emmet_enabled = false;
    paint(application, context, Vec::new());
    assert!(
        application
            .editor_completion
            .candidates(&application.store, view)
            .is_none_or(|items| items
                .iter()
                .all(|candidate| candidate.item.detail.as_deref() != Some("Emmet abbreviation")))
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), original);
    eframe::App::on_exit(application);
    application.runtime.block_on(async {
        application.services.tasks.shutdown().await;
    });
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
