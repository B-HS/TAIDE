use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, key, paint, text_contains, wait,
};
use super::*;

const ORIGINAL: &str = "old\nkeep";
const GENERATED: &str = "new\nnext\nkeep";
const DEADLINE: Duration = Duration::from_secs(10);

#[test]
fn 실제_ai_edit의_다른파일_peek는_source만_수락하고_owner와_디스크를_보존한다() {
    ai_edit_peek(false);
}

#[test]
fn 실제_ai_edit의_팔레트는_다른파일_peek의_현재_source에_지시와_수락을_연결한다() {
    ai_edit_peek(true);
}

fn ai_edit_peek(from_palette: bool) {
    let mut fixture = fixture_with_view_state(
        "--native-locations-cross-file",
        false,
        |application, project, document| {
            let snapshot = application.store.documents().snapshot(document).unwrap();
            application.lsp.as_ref().is_some_and(|lsp| {
                !lsp.location_providers(
                    project,
                    &snapshot,
                    taide_native_editor::symbol_locations::Kind::Definition,
                )
                .is_empty()
            })
        },
        None,
        ORIGINAL,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let owner = body_view(application, &fixture.path, &fixture.tab);
    let owner_document = application.files[&fixture.path].id;
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Request {
            kind: taide_native_editor::symbol_locations::Kind::Definition,
            mode: taide_native_editor::symbol_locations::Mode::Peek,
        }),
    ));
    wait(application, context, "AI edit source peek", |application| {
        application
            .editor_locations
            .current(owner)
            .is_some_and(|session| session.preview.is_some())
    });
    let source = application
        .editor_locations
        .current(owner)
        .unwrap()
        .preview
        .unwrap();
    let document = application.store.views().get(source).unwrap().document;
    assert_ne!(document, owner_document);
    let snapshot = application.store.documents().snapshot(document).unwrap();
    let original = snapshot.rope.to_string();
    application.services.state.settings.write().ai_provider =
        Some(taide_model::ai::AiProviderId::Omlx);
    application.services.state.settings.write().ai_model = Some("fixture".into());
    let captured = Arc::new(std::sync::Mutex::new(Vec::new()));
    let requests = captured.clone();
    let (pending_send, pending_receive) = std::sync::mpsc::channel();
    application.editor_completion.inline_edit =
        crate::editor_ai_edit::State::with_complete(Arc::new(move |_, request| {
            let requests = requests.clone();
            let pending_send = pending_send.clone();
            Box::pin(async move {
                let request_id = request.request_id.clone();
                let should_wait = {
                    let mut requests = requests.lock().unwrap();
                    requests.push(request);
                    requests.len() > 1
                };
                if should_wait {
                    let (reply, result) = tokio::sync::oneshot::channel();
                    pending_send.send(reply).unwrap();
                    return result
                        .await
                        .unwrap_or_else(|_| Err(AppError::Internal("fixture closed".into())));
                }
                Ok(taide_model::ai::AiTextResponse {
                    request_id,
                    text: Some("peek_generated".into()),
                })
            })
        }));
    paint(application, context, Vec::new());
    let body = crate::editor_context_menu::body(context, source).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::Home, egui::Modifiers::COMMAND),
    );
    if from_palette {
        super::selection_ranges_tests::run_palette(application, context, "ai.inlineEdit");
        assert!(!application.palette.is_open());
    } else {
        paint(
            application,
            context,
            key(
                egui::Key::I,
                egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
            ),
        );
    }
    wait(application, context, "AI edit peek input", |application| {
        application.editor_completion.inline_edit.is_open(
            &application.store,
            &application.services,
            source,
        )
    });
    paint(
        application,
        context,
        vec![egui::Event::Text("rename".into())],
    );
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let deadline = Instant::now() + DEADLINE;
    loop {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        let output = paint(application, context, Vec::new());
        if text_contains(&output, "peek_generated") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let request = captured.lock().unwrap().last().unwrap().clone();
    assert_eq!(request.file_path, fixture.peek_path);
    assert_eq!(request.language, snapshot.metadata.language_id);
    assert_eq!(request.instruction, "rename");
    assert_eq!(request.prefix, "");
    assert!(original.starts_with(&request.selection));
    assert_eq!(format!("{}{}", request.selection, request.suffix), original);
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
    paint(
        application,
        context,
        key(
            egui::Key::Enter,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    let expected = format!("peek_generated{}", request.suffix);
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
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(owner_document)
            .unwrap()
            .rope
            .to_string(),
        ORIGINAL
    );
    assert_eq!(
        std::fs::read_to_string(&fixture.peek_path).unwrap(),
        original
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
    paint(application, context, Vec::new());
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(
            egui::Key::I,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    paint(
        application,
        context,
        vec![egui::Event::Text("cancel".into())],
    );
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let pending = pending_receive.recv_timeout(DEADLINE).unwrap();
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Close),
    ));
    wait(application, context, "AI edit peek close", |application| {
        application.editor_locations.current(owner).is_none()
    });
    let deadline = Instant::now() + DEADLINE;
    while !pending.is_closed() {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
        assert!(
            Instant::now() < deadline,
            "closed peek retained AI edit future"
        );
        std::thread::yield_now();
    }
    assert!(!application.editor_completion.inline_edit.is_open(
        &application.store,
        &application.services,
        source
    ));
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_ai_edit의_cmdi_지시_미리보기_cmdenter_수락과_undo는_디스크를_보존한다() {
    ai_edit_body(false);
}

#[test]
fn 실제_ai_edit의_조합중_enter_escape는_지시를_보존하고_확정뒤_마우스로_수락한다() {
    ai_edit_body(true);
}

fn ai_edit_body(with_ime_and_mouse: bool) {
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        ORIGINAL,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    application.services.state.settings.write().ai_provider =
        Some(taide_model::ai::AiProviderId::Omlx);
    application.services.state.settings.write().ai_model = Some("fixture".into());
    let captured = Arc::new(std::sync::Mutex::new(Vec::new()));
    let requests = captured.clone();
    application.editor_completion.inline_edit =
        crate::editor_ai_edit::State::with_complete(Arc::new(move |_, request| {
            let requests = requests.clone();
            Box::pin(async move {
                let request_id = request.request_id.clone();
                requests.lock().unwrap().push(request);
                Ok(taide_model::ai::AiTextResponse {
                    request_id,
                    text: Some("```rust\nnew\nnext\n```".into()),
                })
            })
        }));
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::Home, egui::Modifiers::COMMAND),
    );
    paint(
        application,
        context,
        key(
            egui::Key::I,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    wait(application, context, "AI edit input", |application| {
        application.editor_completion.inline_edit.is_open(
            &application.store,
            &application.services,
            view,
        )
    });
    let instruction = if with_ime_and_mouse {
        "한글 수정"
    } else {
        "rename"
    };
    if with_ime_and_mouse {
        wait(application, context, "AI instruction IME focus", |_| {
            let Some(editor) = crate::editor_context_menu::body(context, view) else {
                return false;
            };
            context.memory(|memory| {
                memory.focused().is_some_and(|focus| {
                    memory.owns_ime_events(focus)
                        && taide_native_ui::editor_ai_edit::is_control(context, editor, focus)
                })
            })
        });
        paint(
            application,
            context,
            vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "한".into(),
                active_range_chars: None,
            })],
        );
        let mut events = key(egui::Key::Enter, egui::Modifiers::NONE);
        events.extend(key(egui::Key::Escape, egui::Modifiers::NONE));
        paint(application, context, events);
        assert!(captured.lock().unwrap().is_empty());
        assert!(application.editor_completion.inline_edit.is_open(
            &application.store,
            &application.services,
            view,
        ));
        paint(
            application,
            context,
            vec![egui::Event::Ime(egui::ImeEvent::Commit(instruction.into()))],
        );
        assert_eq!(
            application
                .editor_completion
                .inline_edit
                .session(&application.store, &application.services, view,)
                .unwrap()
                .instruction,
            instruction
        );
    } else {
        paint(
            application,
            context,
            vec![egui::Event::Text(instruction.into())],
        );
    }
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let deadline = Instant::now() + DEADLINE;
    loop {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        let output = paint(application, context, Vec::new());
        if text_contains(&output, "new") && text_contains(&output, "next") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].selection, "old");
    assert_eq!(requests[0].instruction, instruction);
    assert_eq!(requests[0].file_path, fixture.path);
    drop(requests);
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
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    if with_ime_and_mouse {
        let label =
            taide_native_ui::presentation::message(&application.locale, "ai.inlineEditAccept", &[]);
        let output = paint(application, context, Vec::new());
        let text = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
                _ => None,
            })
            .expect("visible localized AI edit accept button");
        let position = text.pos + text.galley.size() * 0.5;
        let pointer = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        paint(
            application,
            context,
            vec![egui::Event::PointerMoved(position), pointer(true)],
        );
        paint(application, context, vec![pointer(false)]);
    } else {
        paint(
            application,
            context,
            key(
                egui::Key::Enter,
                egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
            ),
        );
    }
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        GENERATED,
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
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_ai_edit의_재정의_명령은_입력에_도달하고_loading_escape는_취소한다() {
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        ORIGINAL,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    {
        let mut settings = application.services.state.settings.write();
        settings.ai_provider = Some(taide_model::ai::AiProviderId::Omlx);
        settings.ai_model = Some("fixture".into());
        settings.keymap_overrides =
            Some(r#"[{"actionId":"monaco.taide.aiInlineEdit","key":"q","mods":["mod"]}]"#.into());
    }
    let (send, receive) = std::sync::mpsc::channel();
    application.editor_completion.inline_edit =
        crate::editor_ai_edit::State::with_complete(Arc::new(move |_, request| {
            let send = send.clone();
            Box::pin(async move {
                let (reply, receive) = tokio::sync::oneshot::channel();
                send.send((request, reply)).unwrap();
                receive
                    .await
                    .unwrap_or_else(|_| Err(AppError::Internal("fixture closed".into())))
            })
        }));
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::Home, egui::Modifiers::COMMAND),
    );
    paint(
        application,
        context,
        key(
            egui::Key::Q,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    wait(
        application,
        context,
        "AI edit override input",
        |application| {
            application.editor_completion.inline_edit.is_open(
                &application.store,
                &application.services,
                view,
            )
        },
    );
    paint(
        application,
        context,
        vec![egui::Event::Text("rename".into())],
    );
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let (_request, reply) = receive.recv_timeout(DEADLINE).unwrap();
    paint(application, context, Vec::new());
    paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    let deadline = Instant::now() + DEADLINE;
    while !reply.is_closed() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    paint(application, context, Vec::new());
    assert!(application.toasts.is_showing(
        taide_native_ui::toast::Kind::Info,
        &presentation::message(&application.locale, "ai.inlineEditCancelled", &[])
    ));
    assert!(
        application
            .editor_completion
            .inline_edit
            .session(&application.store, &application.services, view)
            .is_none()
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
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
