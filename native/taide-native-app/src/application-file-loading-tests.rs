use super::inlay_hints_tests::{body_view, fixture_with_view_state, paint, wait};
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use taide_model::layout::PaneNode;

const TEXT: &str = "class\n  body\nend";
const BINARY: &[u8] = b"synthetic\0binary";
const AUXILIARY_SLOT: u32 = 1;
const TIER_LINE_WIDTH: usize = 1023;

#[test]
fn 실제_파일_크기_등급은_host_로딩과_본문_입력에서_편집_읽기전용을_구분한다() {
    use super::inlay_hints_tests::key;
    use taide_model::file::{FileSizeTier, LARGE_FILE_BYTES, READ_ONLY_FILE_BYTES};
    for (bytes, tier, is_read_only) in [
        (LARGE_FILE_BYTES, FileSizeTier::Large, false),
        (READ_ONLY_FILE_BYTES, FileSizeTier::ReadOnly, true),
    ] {
        let mut fixture =
            fixture_with_view_state("--native-inlay-hints", false, |_, _, _| true, None, TEXT);
        let path = fixture
            .directory
            .join("project/tier.txt")
            .to_str()
            .unwrap()
            .to_owned();
        let line = format!("{}\n", "x".repeat(TIER_LINE_WIDTH));
        let text = line.repeat(usize::try_from(bytes).unwrap() / line.len());
        assert_eq!(u64::try_from(text.len()).unwrap(), bytes);
        std::fs::write(&path, &text).unwrap();
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        assert!(application.submit(HostCommand::OpenFileTab {
            project: fixture.project.clone(),
            pane: None,
            path: path.clone(),
            preview: false
        }));
        wait(application, context, "actual tier loaded", |application| {
            application.files.contains_key(&path)
        });
        let snapshot = application.controller.snapshot();
        let tab = crate::tabs::tabs_in(&snapshot.layouts[&fixture.project].root)
            .into_iter()
            .find(|tab| matches!(&tab.kind, TabKind::File {path: current} if current == &path))
            .unwrap();
        let view = body_view(application, &path, &tab.id);
        let original = application
            .store
            .documents()
            .snapshot(application.files[&path].id)
            .unwrap();
        assert_eq!(original.metadata.tier, tier);
        assert_eq!(original.metadata.read_only, is_read_only);
        let output = paint(application, context, Vec::new());
        if is_read_only {
            let label = presentation::message(&application.locale, "editor.readOnlyLargeFile", &[]);
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains(&label))));
        }
        let focus = crate::editor_context_menu::body(context, view).unwrap();
        context.memory_mut(|memory| memory.request_focus(focus));
        paint(
            application,
            context,
            key(egui::Key::Home, egui::Modifiers::NONE),
        );
        paint(application, context, vec![egui::Event::Text("Z".into())]);
        let current = application.store.documents().snapshot(original.id).unwrap();
        assert_eq!(current.metadata.tier, tier);
        assert_eq!(current.metadata.read_only, is_read_only);
        if is_read_only {
            assert_eq!(current.revision, original.revision);
            assert_eq!(current.rope.to_string(), text);
            assert!(!current.dirty);
        } else {
            assert!(current.revision > original.revision);
            assert!(current.dirty);
            assert!(current.rope.to_string().starts_with('Z'));
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        eframe::App::on_exit(application);
        assert_eq!(application.services.tasks.tracked_count(), 0);
    }
}

#[test]
fn 실제_거절_캐시는_보조_mirror의_수명_파일_변경과_프로젝트_종료를_따른다() {
    let mut fixture =
        fixture_with_view_state("--native-inlay-hints", false, |_, _, _| true, None, TEXT);
    std::fs::write(&fixture.peek_path, BINARY).unwrap();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: None,
        path: fixture.peek_path.clone(),
        preview: false
    }));
    wait(application, context, "refused cache", |application| {
        application.refused.contains(&fixture.peek_path)
    });
    let mut layout = application.services.state.layouts.read()[&fixture.project].clone();
    let first = crate::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(&tab.kind, TabKind::File {path} if path == &fixture.peek_path))
        .unwrap()
        .id
        .clone();
    let pane = PaneId::new();
    let mirror = TabId::new();
    layout
        .auxiliary_windows
        .push(taide_model::layout::AuxWindowLayout {
            slot: AUXILIARY_SLOT,
            root: PaneNode::Leaf {
                id: pane.clone(),
                tabs: vec![Tab {
                    id: mirror.clone(),
                    kind: TabKind::File {
                        path: fixture.peek_path.clone(),
                    },
                    title: "mirror.rs".into(),
                    pinned: false,
                    preview: false,
                    dirty: false,
                    view_state: None,
                }],
                active: Some(mirror.clone()),
            },
            focused_pane: pane,
        });
    taide_runtime::layout_actions::finish_mutation(
        application.services.events.as_ref(),
        &application.services.state,
        &fixture.project,
        &mut layout,
    );
    application
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout.clone());
    application
        .controller
        .apply_layouts(HashMap::from([(fixture.project.clone(), layout)]));
    application.shell.scope = WindowScope::Auxiliary {
        project: fixture.project.clone(),
        slot: AUXILIARY_SLOT,
    };
    let output = paint(application, context, Vec::new());
    let label = presentation::message(&application.locale, "editor.binaryOrTooLarge", &[]);
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains(&label))));
    application.request_tab_close(first.clone());
    wait(
        application,
        context,
        "refused first tab closed",
        |application| {
            application.pending_tab_close.is_none()
                && !crate::tabs::tabs_in(
                    &application.controller.snapshot().layouts[&fixture.project].root,
                )
                .iter()
                .any(|tab| tab.id == first)
        },
    );
    assert!(application.refused.contains(&fixture.peek_path));
    std::fs::write(&fixture.peek_path, TEXT).unwrap();
    application
        .services
        .events
        .publish(taide_model::app_event::AppEvent::FsChanged {
            project_id: fixture.project.clone(),
            change: taide_model::file::FsChange {
                kind: taide_model::file::FsChangeKind::Modified,
                paths: vec![fixture.peek_path.clone()],
                from_app: false,
            },
        });
    wait(
        application,
        context,
        "refused changed to text",
        |application| application.files.contains_key(&fixture.peek_path),
    );
    assert!(!application.refused.contains(&fixture.peek_path));
    let view = body_view(application, &fixture.peek_path, &mirror);
    let document = application
        .store
        .documents()
        .snapshot(application.store.views().get(view).unwrap().document)
        .unwrap();
    assert_eq!(document.rope.to_string(), TEXT);
    assert!(!document.dirty);
    let rejected = fixture
        .directory
        .join("project/rejected.rs")
        .to_str()
        .unwrap()
        .to_owned();
    std::fs::write(&rejected, BINARY).unwrap();
    application.shell.scope = WindowScope::Main;
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: None,
        path: rejected.clone(),
        preview: false
    }));
    wait(
        application,
        context,
        "refused before project close",
        |application| application.refused.contains(&rejected),
    );
    let services = application.services.clone();
    let dispatch = application.application_ports.dispatch.clone();
    let arguments = serde_json::json!({"projectId": fixture.project});
    let closed = application.runtime.spawn(async move {
        let channels: crate::remote_ws::ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("project close must not create a channel")));
        (dispatch.json)(services, "project_close".into(), arguments, channels).await
    });
    wait(
        application,
        context,
        "refused project removed",
        |application| closed.is_finished() && application.refused.is_empty(),
    );
    assert!(application.runtime.block_on(closed).unwrap().is_ok());
    assert_eq!(std::fs::read(&rejected).unwrap(), BINARY);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_거절_파일은_전용_안내와_외부_열기를_표시하고_문서_모델을_생성하지_않는다() {
    for bytes in [
        None,
        Some(taide_model::file::REFUSED_FILE_BYTES),
        Some(u64::from(u32::MAX) + 1),
    ] {
        let mut fixture =
            fixture_with_view_state("--native-inlay-hints", false, |_, _, _| true, None, TEXT);
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let view = body_view(application, &fixture.path, &fixture.tab);
        let original = application
            .store
            .documents()
            .snapshot(application.store.views().get(view).unwrap().document)
            .unwrap();
        if let Some(bytes) = bytes {
            std::fs::File::create(&fixture.peek_path)
                .unwrap()
                .set_len(bytes)
                .unwrap();
        } else {
            std::fs::write(&fixture.peek_path, BINARY).unwrap();
        }
        assert!(application.submit(HostCommand::OpenFileTab {
            project: fixture.project.clone(),
            pane: None,
            path: fixture.peek_path.clone(),
            preview: false,
        }));
        let had_loading = AtomicBool::new(false);
        wait(application, context, "refused file loaded", |application| {
            if application.loading.contains(&fixture.peek_path) {
                had_loading.store(true, Ordering::Release);
                return false;
            }
            had_loading.load(Ordering::Acquire)
        });
        let output = paint(application, context, Vec::new());
        for key in [
            "editor.cannotOpen",
            "editor.binaryOrTooLarge",
            "editor.openExternally",
        ] {
            let label = presentation::message(&application.locale, key, &[]);
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains(&label))), "missing refused label {key}");
        }
        assert!(!application.files.contains_key(&fixture.peek_path));
        assert!(
            application
                .store
                .documents()
                .find(&taide_native_editor::document::DocumentKey::File(
                    PathBuf::from(&fixture.peek_path)
                ))
                .is_none()
        );
        let current = application.store.documents().snapshot(original.id).unwrap();
        assert_eq!(current.rope.to_string(), TEXT);
        assert_eq!(current.revision, original.revision);
        assert!(!current.dirty);
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
        if let Some(bytes) = bytes {
            assert_eq!(std::fs::metadata(&fixture.peek_path).unwrap().len(), bytes);
        } else {
            assert_eq!(std::fs::read(&fixture.peek_path).unwrap(), BINARY);
        }
    }
}

#[test]
fn 실제_읽기_실패_탭을_마지막으로_닫고_다시_열면_오류_캐시가_정상_본문을_막지_않는다() {
    let mut fixture =
        fixture_with_view_state("--native-inlay-hints", false, |_, _, _| true, None, TEXT);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let failed = TabId::new();
    let failed_path = fixture
        .directory
        .join("project/missing.rs")
        .to_str()
        .unwrap()
        .to_owned();
    let mut layout = application.services.state.layouts.read()[&fixture.project].clone();
    let PaneNode::Leaf { tabs, active, .. } = &mut layout.root else {
        panic!("expected fixture leaf")
    };
    tabs.push(Tab {
        id: failed.clone(),
        kind: TabKind::File {
            path: failed_path.clone(),
        },
        title: "missing.rs".into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    });
    *active = Some(failed.clone());
    taide_runtime::layout_actions::finish_mutation(
        application.services.events.as_ref(),
        &application.services.state,
        &fixture.project,
        &mut layout,
    );
    application
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout.clone());
    application
        .controller
        .apply_layouts(HashMap::from([(fixture.project.clone(), layout)]));
    wait(
        application,
        context,
        "missing file failure",
        |application| application.failed.contains_key(&failed_path),
    );
    application.request_tab_close(failed.clone());
    wait(application, context, "failed tab closed", |application| {
        application.pending_tab_close.is_none()
            && !crate::tabs::tabs_in(
                &application.controller.snapshot().layouts[&fixture.project].root,
            )
            .iter()
            .any(|tab| tab.id == failed)
    });
    assert!(
        !application.failed.contains_key(&failed_path),
        "last failed tab retained a stale error"
    );
    std::fs::write(&failed_path, TEXT).unwrap();
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: None,
        path: failed_path.clone(),
        preview: false
    }));
    wait(
        application,
        context,
        "reopened normal file",
        |application| application.files.contains_key(&failed_path),
    );
    let document = application
        .store
        .documents()
        .snapshot(application.files[&failed_path].id)
        .unwrap();
    assert_eq!(document.rope.to_string(), TEXT);
    assert!(!document.dirty);
    assert_eq!(std::fs::read_to_string(&failed_path).unwrap(), TEXT);
}
