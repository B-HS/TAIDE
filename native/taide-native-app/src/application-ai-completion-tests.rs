use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, fixture_with_view_state_and_secrets,
    fixture_with_view_state_and_setup, key, paint, text_contains, wait,
};
use super::*;

const ORIGINAL: &str = "let value = ";
const GENERATED: &str = "native_generated\nnext_line";
const PAINT_DEADLINE: Duration = Duration::from_secs(10);

#[test]
fn 실제_ai_inline의_후보확장은_미리보기_설정과_독립적으로_표시하고_두번에_수락한다() {
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        "con",
        |application, _, _, _| {
            taide_runtime::snippet_actions::snippet_save(
                &application.services.state,
                "rust.json".into(),
                r#"{"Console":{"prefix":"con","body":"console"}}"#.into(),
            )
            .unwrap();
            application.settings_views.refresh_snippets();
        },
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    wait(
        application,
        context,
        "AI candidate catalog",
        |application| {
            !application
                .settings_views
                .snippet_catalog()
                .files()
                .is_empty()
        },
    );
    {
        let mut settings = application.services.state.settings.write();
        settings.ai_auto_tab_enabled = true;
        settings.ai_provider = Some(taide_model::ai::AiProviderId::Omlx);
        settings.ai_model = Some("fixture".into());
        settings.keymap_overrides = Some(r#"[{"actionId":"monaco.editor.action.inlineSuggest.trigger","key":"q","mods":["mod"]}]"#.into());
    }
    application.editor_completion.inline =
        crate::editor_ai_completion::State::with_complete(Arc::new(|_, request, _| {
            Box::pin(async move {
                Ok(taide_model::ai::AiTextResponse {
                    request_id: request.request_id,
                    text: Some("sole.extra".into()),
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
        if text_contains(&output, ".extra") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    paint(
        application,
        context,
        key(egui::Key::Space, egui::Modifiers::CTRL),
    );
    wait(application, context, "AI candidate list", |application| {
        application
            .editor_completion
            .request(&application.store, view)
            .is_some()
    });
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, ".extra"));
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "con"
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
        "console"
    );
    let ghosts = application
        .editor_completion
        .inline
        .poll(&application.store, view);
    assert_eq!(ghosts[0].parts[0].text, ".extra");
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
        "console.extra"
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), "con");
    application.store.undo(document).unwrap();
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "console"
    );
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_ai_inline의_split은_활성뷰만_표시하고_mirror_수락과_undo를_공유한다() {
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        ORIGINAL,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let first = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    let pane = application
        .store
        .views()
        .get(first)
        .unwrap()
        .key
        .pane
        .clone();
    assert!(application.submit(HostCommand::OpenFileToSide(
        taide_model::layout::OpenTabInSplitRequest {
            project_id: fixture.project.clone(),
            target_pane: pane,
            edge: taide_model::layout::DropEdge::Right,
            kind: TabKind::File {
                path: fixture.path.clone()
            },
            title: "current.rs".into(),
            preview: false,
        }
    )));
    wait(application, context, "AI split mirror", |application| {
        application.store.views().for_document(document).count() > 1
    });
    let second = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.id != first)
        .unwrap()
        .id;
    {
        let mut settings = application.services.state.settings.write();
        settings.ai_auto_tab_enabled = true;
        settings.ai_provider = Some(taide_model::ai::AiProviderId::Omlx);
        settings.ai_model = Some("fixture".into());
        settings.keymap_overrides = Some(r#"[{"actionId":"monaco.editor.action.inlineSuggest.trigger","key":"q","mods":["mod"]}]"#.into());
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
    for source in [first, second] {
        let pane = application
            .store
            .views()
            .get(source)
            .unwrap()
            .key
            .pane
            .clone();
        application
            .controller
            .submit(ShellMutation::FocusPane(pane.clone()))
            .unwrap();
        wait(application, context, "AI split focus", |application| {
            application
                .controller
                .snapshot()
                .layouts
                .get(&fixture.project)
                .is_some_and(|layout| layout.focused_pane == pane)
        });
        paint(application, context, Vec::new());
        let body = crate::editor_context_menu::body(context, source).unwrap();
        context.memory_mut(|memory| memory.request_focus(body));
        paint(
            application,
            context,
            key(egui::Key::End, egui::Modifiers::COMMAND),
        );
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
            assert!(Instant::now() < deadline, "split AI ghost was not painted");
            std::thread::yield_now();
        }
        let other = if source == first { second } else { first };
        assert!(
            application
                .editor_completion
                .inline
                .poll(&application.store, other)
                .is_empty()
        );
        assert!(
            !application
                .editor_completion
                .inline
                .poll(&application.store, source)
                .is_empty()
        );
    }
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
    assert_eq!(
        application.store.views().get(first).unwrap().document,
        application.store.views().get(second).unwrap().document
    );
    application.store.undo(document).unwrap();
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
fn 실제_ai_inline의_peek는_다른_파일_문맥으로_요청하고_소스만_수락한다() {
    ai_inline_peek(false);
}

#[test]
fn 실제_ai_inline의_팔레트는_다른파일_peek의_현재_source에_요청과_수락을_연결한다() {
    ai_inline_peek(true);
}

fn ai_inline_peek(from_palette: bool) {
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
    wait(application, context, "AI cross-file peek", |application| {
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
    let source_document = application.store.views().get(source).unwrap().document;
    assert_ne!(source_document, owner_document);
    let before = application
        .store
        .documents()
        .snapshot(source_document)
        .unwrap();
    let original = before.rope.to_string();
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
    paint(application, context, Vec::new());
    let body = crate::editor_context_menu::body(context, source).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        context,
        key(egui::Key::End, egui::Modifiers::COMMAND),
    );
    if from_palette {
        super::selection_ranges_tests::run_palette(
            application,
            context,
            "monaco.editor.action.inlineSuggest.trigger",
        );
    } else {
        paint(
            application,
            context,
            key(
                egui::Key::Q,
                egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
            ),
        );
    }
    let deadline = Instant::now() + PAINT_DEADLINE;
    loop {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        let output = paint(application, context, Vec::new());
        if text_contains(&output, "native_generated") {
            break;
        }
        assert!(Instant::now() < deadline, "peek AI ghost was not painted");
        std::thread::yield_now();
    }
    let request = requests.lock().unwrap().last().unwrap().clone();
    assert_eq!(request.file_path, fixture.peek_path);
    assert_eq!(request.language, before.metadata.language_id);
    assert_eq!(request.prefix, original);
    assert_eq!(request.suffix, "");
    paint(
        application,
        context,
        key(egui::Key::Tab, egui::Modifiers::NONE),
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(source_document)
            .unwrap()
            .rope
            .to_string(),
        format!("{original}{GENERATED}")
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
    application.store.undo(source_document).unwrap();
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(source_document)
            .unwrap()
            .rope
            .to_string(),
        original
    );
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Close),
    ));
    wait(application, context, "AI peek closed", |application| {
        application.editor_locations.current(owner).is_none()
    });
    assert!(
        application
            .editor_completion
            .inline
            .poll(&application.store, source)
            .is_empty()
    );
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

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
