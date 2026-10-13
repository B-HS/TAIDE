use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, fixture_with_view_state_and_secrets, key, paint,
    text_contains, wait,
};
use super::*;

const ORIGINAL: &str = "let value = ";
const GENERATED: &str = "native_generated\nnext_line";
const PAINT_DEADLINE: Duration = Duration::from_secs(10);

#[test]
fn 실제_ai_inline의_명시_키_재정의는_입력없이_요청하고_수락한다() {
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
        settings.ai_auto_tab_enabled = true;
        settings.ai_provider = Some(taide_model::ai::AiProviderId::Omlx);
        settings.ai_model = Some("fixture".into());
        settings.keymap_overrides = Some(r#"[{"actionId":"monaco.editor.action.inlineSuggest.trigger","key":"q","mods":["mod"]}]"#.into());
    }
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = requests.clone();
    application.editor_completion.inline =
        crate::editor_ai_completion::State::with_complete(Arc::new(move |_, request, _| {
            recorded.lock().unwrap().push(request.clone());
            Box::pin(async move {
                Ok(taide_model::ai::AiTextResponse {
                    request_id: request.request_id,
                    text: Some(GENERATED.into()),
                })
            })
        }));
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::End, egui::Modifiers::COMMAND),
    );
    assert!(requests.lock().unwrap().is_empty());
    paint(
        application,
        context,
        key(
            egui::Key::Q,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    let deadline = Instant::now() + PAINT_DEADLINE;
    loop {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        let output = paint(application, context, Vec::new());
        if text_contains(&output, "native_generated") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "explicit AI ghost was not painted"
        );
        std::thread::yield_now();
    }
    let request = requests.lock().unwrap().last().unwrap().clone();
    assert_eq!(request.prefix, ORIGINAL);
    assert_eq!(request.file_path, fixture.path);
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
        format!("{ORIGINAL}{GENERATED}")
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

struct Credentials;

impl taide_infra::secret::SecretStore for Credentials {
    fn set(&self, _: taide_infra::secret::SecretAccount, _: &str) -> AppResult<()> {
        panic!("credential deletion fixture must not store values");
    }
    fn get(&self, _: taide_infra::secret::SecretAccount) -> AppResult<Option<String>> {
        panic!("credential deletion fixture must not read values");
    }
    fn delete(&self, account: taide_infra::secret::SecretAccount) -> AppResult<()> {
        assert_eq!(account, taide_infra::secret::SecretAccount::AiOmlx);
        Ok(())
    }
}

#[test]
fn 실제_ai_inline의_자격증명_삭제는_표시를_회수하고_원문과_디스크를_보존한다() {
    let secrets = taide_infra::secret::SecretStoreState::with_store(Arc::new(Credentials));
    let mut fixture = fixture_with_view_state_and_secrets(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        ORIGINAL,
        secrets,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    {
        let mut settings = application.services.state.settings.write();
        settings.ai_auto_tab_enabled = true;
        settings.ai_provider = Some(taide_model::ai::AiProviderId::Omlx);
        settings.ai_model = Some("fixture".into());
    }
    application.editor_completion.inline =
        crate::editor_ai_completion::State::with_complete(Arc::new(|_, request, _| {
            Box::pin(async move {
                Ok(taide_model::ai::AiTextResponse {
                    request_id: request.request_id,
                    text: Some(GENERATED.into()),
                })
            })
        }));
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::End, egui::Modifiers::COMMAND),
    );
    paint(application, context, vec![egui::Event::Text(";".into())]);
    let deadline = Instant::now() + PAINT_DEADLINE;
    loop {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        let output = paint(application, context, Vec::new());
        if text_contains(&output, "native_generated") {
            break;
        }
        assert!(Instant::now() < deadline, "AI ghost was not painted");
        std::thread::yield_now();
    }
    application
        .runtime
        .block_on(taide_runtime::ai_actions::ai_clear_token(
            &application.services.secrets,
            taide_model::ai::AiProviderId::Omlx,
        ))
        .unwrap();
    eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    let output = paint(application, context, Vec::new());
    assert!(!text_contains(&output, "native_generated"));
    assert!(
        application
            .editor_completion
            .inline
            .poll(&application.store, view)
            .is_empty()
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        format!("{ORIGINAL};")
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_ai_inline은_주입된_응답을_표시하고_수락전_디스크와_원문을_보존한다() {
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
        settings.ai_auto_tab_enabled = true;
        settings.ai_provider = Some(taide_model::ai::AiProviderId::Omlx);
        settings.ai_model = Some("fixture".into());
    }
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = requests.clone();
    application.editor_completion.inline =
        crate::editor_ai_completion::State::with_complete(Arc::new(move |_, request, _| {
            recorded.lock().unwrap().push(request.clone());
            Box::pin(async move {
                Ok(taide_model::ai::AiTextResponse {
                    request_id: request.request_id,
                    text: Some(GENERATED.into()),
                })
            })
        }));
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::End, egui::Modifiers::COMMAND),
    );
    paint(application, context, vec![egui::Event::Text(";".into())]);
    wait(application, context, "native AI ghost", |application| {
        requests
            .lock()
            .unwrap()
            .last()
            .is_some_and(|request| request.prefix == format!("{ORIGINAL};"))
            && application
                .store
                .documents()
                .snapshot(document)
                .is_ok_and(|snapshot| snapshot.rope.to_string() == format!("{ORIGINAL};"))
    });
    let deadline = Instant::now() + PAINT_DEADLINE;
    loop {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        let output = paint(application, context, Vec::new());
        if text_contains(&output, "native_generated") && text_contains(&output, "next_line") {
            break;
        }
        assert!(Instant::now() < deadline, "AI ghost was not painted");
        std::thread::yield_now();
    }
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    let request = requests.lock().unwrap().last().unwrap().clone();
    assert_eq!(request.file_path, fixture.path);
    assert_eq!(
        request.owner,
        application.store.views().get(view).unwrap().key.window
    );
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
        format!("{ORIGINAL};{GENERATED}")
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), ORIGINAL);
    application.store.undo(document).unwrap();
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        format!("{ORIGINAL};")
    );
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
