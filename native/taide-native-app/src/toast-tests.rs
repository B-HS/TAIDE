use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind};
use taide_model::file::{FileSizeTier, OpenedFile};
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_model::locale::ResolvedLocale;
use taide_model::paths::AppPaths;
use taide_model::theme::ThemeType;
use taide_model::tree::{TreeEntryKind, TreeRow, TreeRowPage};
use taide_native_editor::document::EditorError;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_ui::toast::Kind;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

use super::*;
use crate::host::{HostBridge, HostCommand, HostReply};

const SETTLE: Duration = Duration::from_millis(400);
const HOST_REPLY_DEADLINE: Duration = Duration::from_secs(3);
const STORE_CAPACITY: usize = 4;
const DOCUMENT_BYTES: usize = 1024;
const READ_ONLY_CONTENT: &str = "synthetic";

struct Events;

impl EventSink for Events {
    fn publish(&self, _: AppEvent) {}
}

struct Directory(std::path::PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

async fn host_reply(bridge: &mut HostBridge, ready: &Notify) -> HostReply {
    tokio::time::timeout(HOST_REPLY_DEADLINE, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap()
}

fn locale() -> ResolvedLocale {
    ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: BTreeMap::from([
            ("app.openProjectFirst".into(), "Open a project first".into()),
            (
                "tab.pinnedCloseBlocked".into(),
                "Pinned tab. Unpin {{title}} before closing it.".into(),
            ),
            (
                "common.copyFailed".into(),
                "Couldn't copy to clipboard".into(),
            ),
            (
                "terminal.openLinkFailed".into(),
                "Failed to open link".into(),
            ),
            ("common.retry".into(), "Retry".into()),
            (
                "editor.readOnlySaveBlocked".into(),
                "Not saved: this file is read-only".into(),
            ),
            ("test.failure".into(), "Cannot write {{target}}".into()),
        ]),
    }
}

fn frame(toasts: &mut Toasts, context: &egui::Context, now: Instant, events: Vec<egui::Event>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 800.0),
        )),
        events,
        ..Default::default()
    };
    let mut output = context.run_ui(input, |ui| {
        toasts.tick(ui.ctx(), now, true);
        toasts
            .show(ui.ctx(), ThemeType::Dark, "bottom-right", now, true)
            .unwrap();
    });
    output.textures_delta.clear();
}

#[test]
fn 안내_toast는_원본과_같은_종류와_카탈로그_문구로_한번씩_발행된다() {
    let locale = locale();
    let now = Instant::now();
    let mut toasts = Toasts::with_actions().unwrap();
    open_project_first(&mut toasts, &locale, now);
    pinned_close_blocked(&mut toasts, &locale, "synthetic.rs", now);
    copy_failed(&mut toasts, &locale, now);
    open_link_failed(&mut toasts, &locale, now);
    let shown = toasts.inspection();
    assert_eq!(shown.len(), 4);
    assert_eq!(shown[3].kind, Kind::Info);
    assert_eq!(shown[3].title, "Open a project first");
    assert_eq!(shown[2].kind, Kind::Warning);
    assert_eq!(
        shown[2].title,
        "Pinned tab. Unpin synthetic.rs before closing it."
    );
    assert_eq!(shown[1].kind, Kind::Error);
    assert_eq!(shown[1].title, "Couldn't copy to clipboard");
    assert_eq!(shown[0].kind, Kind::Error);
    assert_eq!(shown[0].title, "Failed to open link");
    assert!(
        shown
            .iter()
            .all(|toast| toast.description.is_none() && toast.action_label.is_none())
    );
}

#[test]
fn 비동기_실패_toast는_로컬라이즈된_오류문구를_응답마다_한번_발행한다() {
    let locale = locale();
    let now = Instant::now();
    let mut toasts = Toasts::with_actions().unwrap();
    let translated = AppError::localized(AppErrorKind::Io, "test.failure", "fallback")
        .with_arg("target", "synthetic");
    toasts.ipc_error(&locale, &translated, now);
    toasts.ipc_error(
        &locale,
        &AppError::Forbidden("synthetic host failure".into()),
        now,
    );
    let shown = toasts.inspection();
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[1].kind, Kind::Error);
    assert_eq!(shown[1].title, "Cannot write synthetic");
    assert_eq!(shown[0].title, "synthetic host failure");
    assert_eq!(
        describe_error(&locale, &translated),
        "Cannot write synthetic"
    );
}

#[test]
fn 반복_보고되는_실패는_같은_toast가_살아있는_동안_한번만_발행한다() {
    let locale = locale();
    let now = Instant::now();
    let context = egui::Context::default();
    let mut toasts = Toasts::with_actions().unwrap();
    let closing = AppError::Forbidden("synthetic host is closing".into());
    for _ in 0..3 {
        error_once(&mut toasts, &locale, &closing, now);
    }
    assert_eq!(toasts.inspection().len(), 1);
    error_once(
        &mut toasts,
        &locale,
        &AppError::Internal("synthetic queue is full".into()),
        now,
    );
    assert_eq!(toasts.inspection().len(), 2);
    let expired = now + Duration::from_secs(5);
    frame(&mut toasts, &context, expired, Vec::new());
    assert!(toasts.inspection().iter().all(|toast| toast.is_dismissed));
    error_once(&mut toasts, &locale, &closing, expired);
    let live = toasts
        .inspection()
        .into_iter()
        .filter(|toast| !toast.is_dismissed)
        .collect::<Vec<_>>();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].title, "synthetic host is closing");
}

#[tokio::test]
async fn 터미널_선택_복사_실패는_toast를_만들지_않고_탐색기_복사_실패만_toast가_된다() {
    let directory =
        Directory(std::env::temp_dir().join(format!("taide-toast-copy-{}", ProjectId::new())));
    std::fs::create_dir_all(&directory.0).unwrap();
    let state = AppState::new(AppPaths::new(directory.0.clone()));
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = crate::bootstrap::services(state, tasks.clone(), Arc::new(Events));
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut bridge = HostBridge::connect_with_clipboard(
        services,
        Arc::new(move || signal.notify_one()),
        Arc::new(|_: &str| {
            Err(AppError::Internal(
                "synthetic clipboard write failed".into(),
            ))
        }),
    )
    .unwrap();
    let locale = locale();
    let now = Instant::now();
    let mut toasts = Toasts::with_actions().unwrap();
    bridge
        .submit(HostCommand::CopyTerminalSelection(
            "synthetic selection".into(),
        ))
        .unwrap();
    let HostReply::TerminalSelectionCopied { result } = host_reply(&mut bridge, &ready).await
    else {
        panic!("expected terminal selection copy reply")
    };
    assert!(result.is_err());
    copy_finished(&mut toasts, &locale, CopyOrigin::Terminal, &result, now);
    assert!(toasts.inspection().is_empty());
    bridge
        .submit(HostCommand::CopyText("/synthetic/root/file.rs".into()))
        .unwrap();
    let HostReply::CopiedText { result } = host_reply(&mut bridge, &ready).await else {
        panic!("expected explorer copy reply")
    };
    assert!(result.is_err());
    copy_finished(&mut toasts, &locale, CopyOrigin::Explorer, &result, now);
    copy_finished(&mut toasts, &locale, CopyOrigin::Explorer, &Ok(()), now);
    let shown = toasts.inspection();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].kind, Kind::Error);
    assert_eq!(shown[0].title, "Couldn't copy to clipboard");
    bridge.disconnect().await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn 읽기전용_파일의_명시_저장_요청은_로컬라이즈된_차단_문구_toast를_요청마다_발행한다() {
    let locale = locale();
    let now = Instant::now();
    let mut toasts = Toasts::with_actions().unwrap();
    let mut store = EditorStore::new(EditorLimits {
        max_documents: STORE_CAPACITY,
        max_views: STORE_CAPACITY,
        max_undo_groups: STORE_CAPACITY,
        max_document_bytes: DOCUMENT_BYTES,
    })
    .unwrap();
    let path = std::env::temp_dir().join("taide-toast-read-only.txt");
    let document = store
        .open_file(
            path.clone(),
            OpenedFile {
                path: path.to_str().unwrap().into(),
                content: READ_ONLY_CONTENT.into(),
                language_id: "plaintext".into(),
                byte_size: READ_ONLY_CONTENT.len() as u32,
                line_count: 1,
                tier: FileSizeTier::Normal,
                read_only: true,
                encoding_lossy: true,
                modified_ms: 0.0,
                editor_config: Default::default(),
            },
        )
        .unwrap();
    let tab = Tab {
        id: TabId::new(),
        kind: TabKind::File {
            path: path.to_str().unwrap().into(),
        },
        title: "taide-toast-read-only.txt".into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    for _ in 0..2 {
        let Err(error) = crate::save::keymap_request(&tab, Some(document), &mut store) else {
            panic!("expected read-only save refusal")
        };
        assert_eq!(error, EditorError::ReadOnly);
        save_failed(&mut toasts, &locale, error, false, now);
    }
    let shown = toasts.inspection();
    assert_eq!(shown.len(), 2);
    assert!(shown.iter().all(
        |toast| toast.kind == Kind::Error && toast.title == "Not saved: this file is read-only"
    ));
    assert_eq!(
        save_error(EditorError::ReadOnly).kind(),
        AppErrorKind::Forbidden
    );
}

#[test]
fn 읽기전용_자동_저장_실패는_toast를_만들지_않고_그밖의_자동_저장_실패는_한번만_발행한다() {
    let locale = locale();
    let now = Instant::now();
    let mut toasts = Toasts::with_actions().unwrap();
    save_failed(&mut toasts, &locale, EditorError::ReadOnly, true, now);
    assert!(toasts.inspection().is_empty());
    for _ in 0..2 {
        save_failed(&mut toasts, &locale, EditorError::StaleRevision, true, now);
    }
    assert_eq!(toasts.inspection().len(), 1);
    for _ in 0..2 {
        save_failed(&mut toasts, &locale, EditorError::StaleRevision, false, now);
    }
    let shown = toasts.inspection();
    assert_eq!(shown.len(), 3);
    assert!(shown.iter().all(|toast| toast.kind == Kind::Error));
}

#[test]
fn 항목_실패_toast의_재시도는_실은_식별자를_한번만_돌려준다() {
    let locale = locale();
    let context = egui::Context::default();
    let now = Instant::now();
    let mut toasts = Toasts::with_actions().unwrap();
    let retry = Action::RetryCreate {
        project: ProjectId::new(),
        request: CreateRequest {
            token: 7,
            path: "/synthetic/root/new.rs".into(),
            parent: "/synthetic/root".into(),
            kind: TreeEntryKind::File,
        },
    };
    entry_failed(
        &mut toasts,
        &locale,
        "Cannot write synthetic".into(),
        retry.clone(),
        now,
    );
    let shown = toasts.inspection();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].kind, Kind::Error);
    assert_eq!(shown[0].title, "Cannot write synthetic");
    assert_eq!(shown[0].action_label.as_deref(), Some("Retry"));
    frame(&mut toasts, &context, now, Vec::new());
    let now = now + SETTLE;
    frame(&mut toasts, &context, now, Vec::new());
    let button = toasts.inspection()[0].action.unwrap().center();
    for pressed in [true, false] {
        frame(
            &mut toasts,
            &context,
            now,
            vec![
                egui::Event::PointerMoved(button),
                egui::Event::PointerButton {
                    pos: button,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
    assert_eq!(toasts.take_actions(), vec![retry]);
    assert!(toasts.inspection()[0].is_dismissed);
    frame(&mut toasts, &context, now, Vec::new());
    assert!(toasts.take_actions().is_empty());
}

fn row(path: &str, name: &str) -> TreeRow {
    TreeRow {
        path: path.into(),
        name: name.into(),
        kind: TreeEntryKind::File,
        depth: 0,
        expanded: false,
        has_children: false,
    }
}

#[test]
fn 이름변경_재시도는_같은_대상으로_요청을_다시_만들고_진행중에는_중복_제출하지_않는다() {
    let locale = locale();
    let rows = vec![row("/synthetic/root/old.rs", "old.rs")];
    let mut explorer = crate::explorer::Explorer::default();
    explorer.start_rename(&rows[0]);
    explorer.rename.as_mut().unwrap().name = "new.rs".into();
    let failed = explorer.commit_rename(&rows, &locale).unwrap();
    assert!(explorer.retry_rename(&failed, &rows, &locale).is_none());
    explorer.rename_finished(&failed, Err("Cannot write synthetic".into()));
    assert_eq!(
        explorer.rename.as_ref().unwrap().error.as_deref(),
        Some("Cannot write synthetic")
    );
    let retried = explorer.retry_rename(&failed, &rows, &locale).unwrap();
    assert_eq!(retried.from, failed.from);
    assert_eq!(retried.to, "/synthetic/root/new.rs");
    assert_ne!(retried.token, failed.token);
    assert!(explorer.rename.as_ref().unwrap().error.is_none());
    assert!(explorer.retry_rename(&failed, &rows, &locale).is_none());
    explorer.rename_finished(&retried, Ok(()));
    assert!(explorer.rename.is_none());
}

#[test]
fn 생성_재시도는_같은_경로로_요청을_다시_만들고_중복_이름은_다시_검증한다() {
    let locale = locale();
    let page = TreeRowPage {
        rows: vec![row("/synthetic/root/old.rs", "old.rs")],
        total: 1,
    };
    let mut explorer = crate::explorer::Explorer::default();
    explorer.root = Some("/synthetic/root".into());
    assert!(explorer.start_create(TreeEntryKind::File, &page).is_none());
    explorer.create.as_mut().unwrap().input.name = "created.rs".into();
    let failed = explorer.commit_create(&page.rows, &locale).unwrap();
    assert!(
        explorer
            .retry_create(&failed, &page.rows, &locale)
            .is_none()
    );
    explorer.create_finished(&failed, Err("Cannot write synthetic".into()));
    assert_eq!(
        explorer.create.as_ref().unwrap().input.error.as_deref(),
        Some("Cannot write synthetic")
    );
    let retried = explorer.retry_create(&failed, &page.rows, &locale).unwrap();
    assert_eq!(retried.path, "/synthetic/root/created.rs");
    assert_eq!(retried.parent, failed.parent);
    assert_eq!(retried.kind, failed.kind);
    assert_ne!(retried.token, failed.token);
    explorer.create_finished(&retried, Err("Cannot write synthetic".into()));
    let taken = TreeRowPage {
        rows: vec![
            row("/synthetic/root/old.rs", "old.rs"),
            row("/synthetic/root/created.rs", "created.rs"),
        ],
        total: 2,
    };
    assert!(
        explorer
            .retry_create(&retried, &taken.rows, &locale)
            .is_none()
    );
    assert_eq!(
        explorer.create.as_ref().unwrap().input.error.as_deref(),
        Some("explorer.entryNameDuplicate")
    );
    let again = explorer
        .retry_create(&retried, &page.rows, &locale)
        .unwrap();
    explorer.create_finished(&again, Ok(()));
    assert!(explorer.create.is_none());
}
