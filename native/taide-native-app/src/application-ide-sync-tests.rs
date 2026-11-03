use std::time::Duration;

use super::ipc_sync_tests::{Opened, edit, open_project};
use super::notifications_tests::{frame, frame_until};
use super::*;
use taide_model::app_event::AppEvent;
use taide_model::ide::IdeDiagnosticSeverity;

const DIRTY_FILE: &str = "dirty.txt";
const CLEAN_FILE: &str = "clean.txt";
const LOSSY_FILE: &str = "lossy.txt";
const BROKEN_JSON_FILE: &str = "broken.json";
const DISK_TEXT: &str = "disk";
const LOSSY_BYTES: &[u8] = &[b'a', 0xff, 0xfe, b'b'];
const BROKEN_JSON: &str = "{\"a\": }";
const QUIET_FRAMES: usize = 8;
const DEADLINE: Duration = Duration::from_secs(10);

fn request_save(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &ProjectId,
    path: &str,
) -> bool {
    let request = format!("synthetic-save-{}", ProjectId::new());
    let (responder, mut answer) = tokio::sync::oneshot::channel();
    let _owner = application
        .services
        .ide
        .insert_pending_save_owned(request.clone(), taide_ide::store::PendingSave { responder });
    application
        .services
        .events
        .publish(AppEvent::IdeSaveRequested {
            request_id: request,
            project_id: project.clone(),
            path: path.into(),
        });
    let deadline = Instant::now() + DEADLINE;
    loop {
        frame(application, context, true);
        if let Ok(saved) = answer.try_recv() {
            return saved;
        }
        assert!(Instant::now() < deadline, "save answer for {path}");
        std::thread::yield_now();
    }
}

#[test]
fn 실제_app은_ide_저장요청을_열린_탭의_문서로_처리한다() {
    let Opened {
        mut fixture,
        context,
        project,
        root,
        paths,
    } = open_project(
        "ide-save",
        &[
            (DIRTY_FILE, DISK_TEXT.as_bytes()),
            (CLEAN_FILE, DISK_TEXT.as_bytes()),
            (LOSSY_FILE, LOSSY_BYTES),
        ],
    );
    let application = fixture.application.as_mut().unwrap();
    let (dirty, clean, lossy) = (&paths[0], &paths[1], &paths[2]);
    edit(application, dirty, "edited ");
    let expected = application
        .store
        .documents()
        .snapshot(application.files[dirty].id)
        .unwrap()
        .rope
        .to_string();
    assert!(request_save(application, &context, &project, dirty));
    assert_eq!(std::fs::read_to_string(dirty).unwrap(), expected);
    frame_until(
        application,
        &context,
        true,
        "clean after save",
        |application| {
            application
                .store
                .documents()
                .snapshot(application.files[dirty].id)
                .is_ok_and(|snapshot| !snapshot.dirty)
        },
    );

    let clean_modified = std::fs::metadata(clean).unwrap().modified().unwrap();
    assert!(request_save(application, &context, &project, clean));
    assert_eq!(
        std::fs::metadata(clean).unwrap().modified().unwrap(),
        clean_modified
    );

    let lossy_document = application.files[lossy].id;
    assert!(
        application
            .store
            .documents()
            .snapshot(lossy_document)
            .unwrap()
            .metadata
            .read_only
    );
    assert!(request_save(application, &context, &project, lossy));
    assert_eq!(std::fs::read(lossy).unwrap(), LOSSY_BYTES);

    let unopened = root.join("unopened.txt");
    std::fs::write(&unopened, DISK_TEXT).unwrap();
    assert!(request_save(
        application,
        &context,
        &project,
        unopened.to_str().unwrap()
    ));
    assert_eq!(std::fs::read_to_string(&unopened).unwrap(), DISK_TEXT);
}

#[test]
fn 실제_app은_보조_창에만_열린_dirty_탭도_ide_저장요청으로_저장한다() {
    let crate::application::auxiliary_window_tests::SingleWindow {
        mut fixture,
        context,
        project,
        notes,
        ..
    } = crate::application::auxiliary_window_tests::single_window("ide-save");
    let application = fixture.application.as_mut().unwrap();
    let path = match &notes.kind {
        TabKind::File { path } => path.clone(),
        _ => panic!("the auxiliary tab is a file tab"),
    };
    let in_main_tree =
        crate::tabs::tabs_in(&application.services.state.layouts.read()[&project].root)
            .iter()
            .any(|tab| tab.id == notes.id);
    assert!(!in_main_tree);
    frame_until(
        application,
        &context,
        true,
        "auxiliary document view",
        |application| {
            application.files.get(&path).is_some_and(|file| {
                application
                    .store
                    .views()
                    .for_document(file.id)
                    .next()
                    .is_some()
            })
        },
    );
    edit(application, &path, "edited ");
    let document = application.files[&path].id;
    let expected = application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string();
    assert_ne!(std::fs::read_to_string(&path).unwrap(), expected);
    assert!(request_save(application, &context, &project, &path));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
    frame_until(
        application,
        &context,
        true,
        "auxiliary clean after save",
        |application| {
            application
                .store
                .documents()
                .snapshot(document)
                .is_ok_and(|snapshot| !snapshot.dirty)
        },
    );
}

#[test]
fn 실제_app은_ide가_실행중일_때만_진단을_300ms_뒤_push한다() {
    let Opened {
        mut fixture,
        context,
        paths,
        ..
    } = open_project(
        "ide-diagnostics",
        &[(BROKEN_JSON_FILE, BROKEN_JSON.as_bytes())],
    );
    let application = fixture.application.as_mut().unwrap();
    let path = paths[0].clone();
    frame_until(
        application,
        &context,
        true,
        "json diagnostics",
        |application| !crate::ide_sync::diagnostics(&application.lsp_diagnostics).is_empty(),
    );
    for _ in 0..QUIET_FRAMES {
        frame(application, &context, true);
    }
    assert!(application.services.ide.diagnostics(None).is_none());

    let directory = fixture.directory.clone();
    let application = fixture.application.as_mut().unwrap();
    assert!(
        application
            .services
            .ide
            .mark_started(
                0,
                String::new(),
                directory.join("synthetic-ide"),
                application.runtime.handle().spawn(std::future::pending()),
            )
            .is_some()
    );
    let started = Instant::now();
    frame_until(application, &context, true, "pushed", |application| {
        application
            .services
            .ide
            .diagnostics(Some(&path))
            .is_some_and(|items| !items.is_empty())
    });
    assert!(started.elapsed() >= crate::ide_sync::IDE_DIAGNOSTICS_PUSH_DEBOUNCE);
    let pushed = application.services.ide.diagnostics(Some(&path)).unwrap();
    let expected = crate::ide_sync::diagnostics(&application.lsp_diagnostics);
    assert_eq!(pushed, expected);
    let stored: Vec<_> = application
        .lsp_diagnostics
        .batches()
        .flat_map(|batch| batch.diagnostics.iter())
        .collect();
    assert_eq!(pushed.len(), stored.len());
    for (item, diagnostic) in pushed.iter().zip(stored) {
        assert_eq!(item.path, path);
        assert_eq!(item.severity, IdeDiagnosticSeverity::Error);
        assert_eq!(
            (
                item.start_line,
                item.start_character,
                item.end_line,
                item.end_character
            ),
            (
                diagnostic.range.start.line,
                diagnostic.range.start.character,
                diagnostic.range.end.line,
                diagnostic.range.end.character
            )
        );
        assert_eq!((item.start_line, item.end_line), (0, 0));
        assert_eq!(item.message, diagnostic.message);
        assert_eq!(item.source, diagnostic.source);
    }
}
