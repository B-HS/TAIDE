use std::time::Duration;

use taide_model::{
    app_event::AppEvent, ids::ProjectId, paths::AppPaths, project::Project,
    terminal::PtySpawnOptions,
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::time::timeout;

use super::*;

const DEADLINE: Duration = Duration::from_secs(3);
const SESSION_COUNT: usize = 2;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const HISTORY: usize = 128;
const FRAME_BYTES: usize = 256 * 1024;
const FRAME_COUNT: usize = 64;
const FRAME_VISITS: usize = 4096;

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _event: AppEvent) {}
}

struct Fixture {
    services: Arc<AppServices>,
    tasks: TaskSupervisor,
    hub: Hub,
    sessions: Vec<Arc<Session>>,
}

impl Fixture {
    async fn new() -> Self {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-prepared-input-{}", ProjectId::new())),
        ));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: env!("CARGO_MANIFEST_DIR").into(),
                name: "synthetic input".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let services = crate::bootstrap::services(state, tasks.clone(), Arc::new(Sink));
        let hub = Hub::new(
            services.clone(),
            crate::terminal_host::Limits {
                sessions: SESSION_COUNT,
                core: Default::default(),
                frames: crate::terminal_frames::Limits {
                    bytes: FRAME_BYTES,
                    count: FRAME_COUNT,
                    visits: FRAME_VISITS,
                },
                writer: crate::terminal_writer::Limits {
                    bytes: FRAME_BYTES,
                    count: 1,
                },
            },
        )
        .unwrap();
        let mut sessions = Vec::new();
        for _ in 0..SESSION_COUNT {
            let id = hub
                .spawn(
                    PtySpawnOptions {
                        project_id: project.clone(),
                        cwd: env!("CARGO_MANIFEST_DIR").into(),
                        shell: Some("/bin/cat".into()),
                        cols: COLUMNS,
                        rows: ROWS,
                        scrollback_bytes: None,
                    },
                    HISTORY,
                    async { Vec::new() },
                    crate::terminal_dispatch::EffectPorts {
                        command_colors: Default::default(),
                        updated: Arc::new(|| {}),
                        color: Arc::new(|_| Ok(Rgb { r: 0, g: 0, b: 0 })),
                        geometry: Arc::new(|| {
                            Ok(WindowSize {
                                num_cols: COLUMNS,
                                num_lines: ROWS,
                                cell_width: 1,
                                cell_height: 1,
                            })
                        }),
                        event: Arc::new(|_| Ok(())),
                        stream: Arc::new(|_| Ok(())),
                    },
                )
                .await
                .unwrap();
            sessions.push(hub.get(&id).unwrap());
        }
        Self {
            services,
            tasks,
            hub,
            sessions,
        }
    }

    async fn finish(self) {
        for session in &self.sessions {
            if self.hub.get(session.id()).is_some() {
                timeout(DEADLINE, self.hub.close(session.id()))
                    .await
                    .unwrap()
                    .unwrap();
            }
        }
        self.services.terminal.shutdown();
        timeout(DEADLINE, self.services.terminal.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
        timeout(DEADLINE, self.tasks.shutdown()).await.unwrap();
        assert_eq!(self.tasks.tracked_count(), 0);
    }
}

fn epoch(session: &Session) -> u64 {
    session
        .snapshot(|snapshot| snapshot.core.selection_stamp().unwrap().input_epoch)
        .unwrap()
}

fn prepare(session: &Session, input: NativeInput<'_>, capacity: usize) -> PreparedInput {
    match session.prepare_input(input, capacity).unwrap() {
        InputPreparation::Prepared(prepared) => prepared,
        InputPreparation::Local(action) => panic!("expected prepared input, got {action:?}"),
    }
}

#[tokio::test]
async fn prepared_input은_소유_입장_퇴역과_재시도_epoch를_보존한다() {
    const CAPACITY: usize = 64;
    let fixture = Fixture::new().await;
    let source = &fixture.sessions[0];
    let foreign = &fixture.sessions[1];
    let initial = epoch(source);
    let foreign_initial = epoch(foreign);
    assert!(
        source
            .prepare_input(NativeInput::CommittedText("too-long"), 1)
            .is_err()
    );
    assert_eq!(epoch(source), initial);
    assert!(
        foreign
            .admit_prepared_input(prepare(source, NativeInput::CommittedText("x"), CAPACITY))
            .is_err()
    );
    assert_eq!(epoch(foreign), foreign_initial);
    assert!(
        source
            .submit_prepared_input(
                &fixture.services,
                prepare(source, NativeInput::CommittedText("x"), CAPACITY),
                CAPACITY,
                true
            )
            .is_err()
    );
    let captured = prepare(source, NativeInput::CommittedText("x\n"), CAPACITY);
    assert_eq!(epoch(source), initial);
    let admitted = source.admit_prepared_input(captured).unwrap();
    assert_eq!(epoch(source), initial + 1);
    let pending = match source
        .submit_prepared_input(&fixture.services, admitted, CAPACITY, true)
        .unwrap()
    {
        InputResult::Pending(pending) => pending,
        _ => panic!("deferred input did not produce a pending write"),
    };
    assert_eq!(epoch(source), initial + 1);
    let receipt = match source.retry_input(&fixture.services, pending).unwrap() {
        InputResult::Write(receipt) => receipt,
        _ => panic!("empty writer did not admit the pending write"),
    };
    timeout(DEADLINE, receipt.wait()).await.unwrap().unwrap();
    assert_eq!(epoch(source), initial + 1);
    let focus = source
        .admit_prepared_input(prepare(source, NativeInput::Focus(true), CAPACITY))
        .unwrap();
    assert!(matches!(
        source
            .submit_prepared_input(&fixture.services, focus, CAPACITY, false)
            .unwrap(),
        InputResult::Local(InputAction::Ignore)
    ));
    assert_eq!(epoch(source), initial + 1);
    let wrong = source
        .admit_prepared_input(prepare(source, NativeInput::CommittedText("y"), CAPACITY))
        .unwrap();
    assert!(
        foreign
            .submit_prepared_input(&fixture.services, wrong, CAPACITY, false)
            .is_err()
    );
    assert_eq!(epoch(foreign), foreign_initial);
    let retired_admission = prepare(source, NativeInput::CommittedText("z"), CAPACITY);
    let retired_submission = source
        .admit_prepared_input(prepare(source, NativeInput::CommittedText("z"), CAPACITY))
        .unwrap();
    timeout(DEADLINE, fixture.hub.close(source.id()))
        .await
        .unwrap()
        .unwrap();
    assert!(
        source
            .prepare_input(NativeInput::CommittedText("z"), CAPACITY)
            .is_err()
    );
    assert!(source.admit_prepared_input(retired_admission).is_err());
    assert!(
        source
            .submit_prepared_input(&fixture.services, retired_submission, CAPACITY, false)
            .is_err()
    );
    fixture.finish().await;
}

#[tokio::test]
async fn 입력_오류는_성공한_후속_입력과_write_뒤에_해제된다() {
    let fixture = Fixture::new().await;
    let source = &fixture.sessions[0];
    let mut view = View {
        session: source.id().into(),
        ..Default::default()
    };
    let oversized = "x".repeat(INPUT_BYTES + 1);
    submit_input(
        &mut view,
        source,
        &fixture.services,
        NativeInput::CommittedText(&oversized),
    );
    assert!(view.error.is_some());
    submit_input(
        &mut view,
        source,
        &fixture.services,
        NativeInput::Focus(true),
    );
    assert!(view.error.is_some());
    submit_input(
        &mut view,
        source,
        &fixture.services,
        NativeInput::CommittedText("x"),
    );
    assert_eq!(view.error, None);
    view.outbox.lock().unwrap().error = Some("synthetic write failure".into());
    timeout(DEADLINE, async {
        loop {
            let mut outbox = view.outbox.lock().unwrap();
            outbox.poll(source, &fixture.services);
            if outbox.receipts.is_empty() {
                break;
            }
            drop(outbox);
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(view.outbox.lock().unwrap().error, None);
    fixture.finish().await;
}

#[tokio::test]
async fn staged_input은_count_byte_focus_상한을_입장전에_검사한다() {
    let fixture = Fixture::new().await;
    let source = &fixture.sessions[0];
    let initial = epoch(source);
    let order = |index, phase| WireOrder {
        viewport: egui::ViewportId::ROOT,
        event: EventOrder { frame: 0, index },
        phase,
    };
    let mut outbox = Outbox::default();
    for index in 0..INPUT_RECEIPTS {
        outbox
            .stage(
                source,
                NativeInput::CommittedText("x"),
                order(index, WirePhase::Input),
            )
            .unwrap();
    }
    assert_eq!(epoch(source), initial + INPUT_RECEIPTS as u64);
    assert_eq!(outbox.capacity(), 0);
    assert!(
        outbox
            .stage(
                source,
                NativeInput::CommittedText("not-admitted"),
                order(INPUT_RECEIPTS, WirePhase::Input)
            )
            .is_err()
    );
    assert_eq!(epoch(source), initial + INPUT_RECEIPTS as u64);
    assert!(matches!(
        outbox
            .stage(
                source,
                NativeInput::Preedit("local-only"),
                order(INPUT_RECEIPTS, WirePhase::Input)
            )
            .unwrap(),
        Some(InputAction::Ignore)
    ));
    for index in 0..INPUT_RECEIPTS {
        outbox
            .stage(
                source,
                NativeInput::Focus(index % 2 == 0),
                order(index, WirePhase::FocusGain),
            )
            .unwrap();
    }
    assert!(
        outbox
            .stage(
                source,
                NativeInput::Focus(false),
                order(INPUT_RECEIPTS, WirePhase::FocusLoss)
            )
            .is_err()
    );
    assert_eq!(epoch(source), initial + INPUT_RECEIPTS as u64);
    assert!(outbox.staged_bytes <= QUEUED_INPUT_BYTES);
    assert!(outbox.staged_focus_bytes <= FOCUS_QUEUE_BYTES);
    outbox.clear();
    assert!(!outbox.is_pending());
    assert_eq!(outbox.staged_bytes, 0);
    assert_eq!(outbox.staged_focus_bytes, 0);
    let text = "x".repeat(INPUT_BYTES);
    let capacity = QUEUED_INPUT_BYTES / (INPUT_BYTES + size_of::<StagedInput>());
    assert!(capacity < INPUT_RECEIPTS);
    for index in 0..capacity {
        outbox
            .stage(
                source,
                NativeInput::CommittedText(&text),
                order(index, WirePhase::Input),
            )
            .unwrap();
    }
    let accepted = epoch(source);
    assert!(
        outbox
            .stage(
                source,
                NativeInput::CommittedText(&text),
                order(capacity, WirePhase::Input)
            )
            .is_err()
    );
    assert_eq!(epoch(source), accepted);
    assert_eq!(accepted, initial + (INPUT_RECEIPTS + capacity) as u64);
    assert!(outbox.staged_bytes <= QUEUED_INPUT_BYTES);
    outbox.clear();
    fixture.finish().await;
}
