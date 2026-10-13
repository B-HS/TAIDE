use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, key, paint, text_contains, wait,
};
use super::*;

const ORIGINAL: &str = "old\nkeep";
const GENERATED: &str = "new\nnext\nkeep";
const DEADLINE: Duration = Duration::from_secs(10);

#[test]
fn 실제_ai_edit의_cmdi_지시_미리보기_cmdenter_수락과_undo는_디스크를_보존한다() {
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
        if text_contains(&output, "new") && text_contains(&output, "next") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].selection, "old");
    assert_eq!(requests[0].instruction, "rename");
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
    paint(
        application,
        context,
        key(
            egui::Key::Enter,
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
