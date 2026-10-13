use super::inlay_hints_tests::{body_view, fixture_with_language, key, paint, text_contains, wait};
use super::*;

const ORIGINAL: &str = "div.foo>span";
const EXPECTED: &str = "<div class=\"foo\"><span></span></div>";

#[test]
fn 실제_emmet는_html파일_공급_표시_수락_undo와_설정변경을_연결한다() {
    let mut fixture = fixture_with_language(ORIGINAL, "html");
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
        "html"
    );
    let state = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: ORIGINAL.len(),
                    head: ORIGINAL.len(),
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
    wait(
        application,
        context,
        "Emmet HTML candidate",
        |application| {
            application
                .editor_completion
                .candidates(&application.store, view)
                .is_some_and(|items| {
                    items.iter().any(|candidate| {
                        candidate.item.detail.as_deref() == Some("Emmet abbreviation")
                    })
                })
        },
    );
    let snapshot = application.store.documents().snapshot(document).unwrap();
    let tokens = taide_native_editor::language_configuration::LineSyntax::accurate_tokens(
        &crate::editor_syntax::SyntaxLease::new(&mut application.editor_syntax, document),
        &snapshot,
        0,
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
    assert!(text_contains(&output, ORIGINAL));
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
        EXPECTED
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
        ORIGINAL
    );
    let state = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: ORIGINAL.len(),
                    head: ORIGINAL.len(),
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
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    eframe::App::on_exit(application);
    application.runtime.block_on(async {
        application.services.tasks.shutdown().await;
    });
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
