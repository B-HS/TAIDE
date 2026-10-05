use std::future::{Future, poll_fn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::{Duration, Instant};

use taide_infra::terminal_scan::ScanEvent;
use taide_model::{
    agent::AgentActivity, app_event::AppEvent, error::AppError, ids::ProjectId, paths::AppPaths,
};
use taide_native_app::{
    bootstrap::services,
    terminal_dispatch::{Dispatcher, EffectPorts, ObservePorts},
    terminal_frames, terminal_writer,
};
use taide_native_terminal::{Rgb, Size, TerminalEvent, WindowSize, session::SharedTerminal};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use taide_terminal::metadata::TerminalSessionMetadata;
use tokio::time::timeout;

const BYTES: usize = 64 * 1024;
const VISITS: usize = 1024;
const COUNT: usize = 8;
const COLUMNS: u16 = 20;
const ROWS: u16 = 4;
const HISTORY: usize = 8;
const CELL_WIDTH: u16 = 8;
const CELL_HEIGHT: u16 = 16;
const COMMAND_MS: u64 = 5500;
const OLD_OUTPUT: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(3);
const SESSION: &str = "synthetic-native-dispatch";
const AGENT: &str = "claude";

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);
impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

fn ports() -> EffectPorts {
    EffectPorts {
        command_colors: Default::default(),
        updated: Arc::new(|| {}),
        color: Arc::new(|index| {
            assert_eq!(index, 1);
            Ok(Rgb { r: 1, g: 2, b: 3 })
        }),
        geometry: Arc::new(|| {
            Ok(WindowSize {
                num_cols: COLUMNS,
                num_lines: ROWS,
                cell_width: CELL_WIDTH,
                cell_height: CELL_HEIGHT,
            })
        }),
        event: Arc::new(|event| {
            assert!(matches!(event, TerminalEvent::Bell));
            Ok(())
        }),
        stream: Arc::new(|_| Ok(())),
    }
}

#[tokio::test]
async fn query_pressure는_진행중인_write_뒤에_응답하고_dispatch를_실패시키지_않는다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-query-pressure-{}", ProjectId::new())),
    ));
    let services = services(state, tasks.clone(), Arc::new(Sink::default()));
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let (sender, mut receiver) = terminal_frames::channel(terminal_frames::Limits {
        bytes: BYTES,
        count: COUNT,
        visits: VISITS,
    })
    .unwrap();
    let writes = Arc::new(Mutex::new(Vec::new()));
    let observed = writes.clone();
    let (release, blocked) = std::sync::mpsc::channel::<()>();
    let blocked = Mutex::new(blocked);
    let (entered, ready) = tokio::sync::oneshot::channel();
    let entered = Mutex::new(Some(entered));
    let (writer, worker) = terminal_writer::Writer::start(
        &tasks,
        terminal_writer::Limits {
            bytes: BYTES,
            count: 1,
        },
        move |bytes| {
            if bytes == b"occupied" {
                if let Some(entered) = entered.lock().unwrap().take() {
                    let _ = entered.send(());
                }
                blocked
                    .lock()
                    .unwrap()
                    .recv()
                    .map_err(|_| AppError::Internal("synthetic query gate closed".into()))?;
            }
            observed.lock().unwrap().push(bytes.to_vec());
            Ok(())
        },
    )
    .unwrap();
    let first = writer.submit(b"occupied".to_vec()).unwrap();
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    sender
        .submit(terminal.advance(b"\x1b]4;1;?\x07").unwrap(), Instant::now())
        .unwrap();
    let delivery = receiver.recv().await.unwrap().unwrap();
    let metadata = Arc::new(TerminalSessionMetadata::new(
        ProjectId::new(),
        "/synthetic".into(),
        "synthetic".into(),
    ));
    let mut dispatcher = Dispatcher::new(SESSION.into(), metadata);
    let effects = ports();
    let mut consume = Box::pin(dispatcher.consume(&delivery, &services, &writer, &effects));
    let early = poll_fn(|context| Poll::Ready(consume.as_mut().poll(context))).await;
    let was_pending = early.is_pending();
    release.send(()).unwrap();
    timeout(TIMEOUT, first.wait()).await.unwrap().unwrap();
    let result = match early {
        Poll::Pending => timeout(TIMEOUT, consume).await.unwrap(),
        Poll::Ready(result) => result,
    };
    writer.close();
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(
        was_pending,
        "transient writer pressure failed the query instead of waiting"
    );
    assert!(result.is_ok(), "query dispatch did not recover: {result:?}");
    assert_eq!(
        *writes.lock().unwrap(),
        [
            b"occupied".to_vec(),
            b"\x1b]4;1;rgb:0101/0202/0303\x07".to_vec()
        ]
    );
}

#[tokio::test]
async fn color_query는_같은_batch의_reset_이전_색을_보존한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-query-{}", ProjectId::new())),
    ));
    let services = services(state, tasks.clone(), Arc::new(Sink::default()));
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let (sender, mut receiver) = terminal_frames::channel(terminal_frames::Limits {
        bytes: BYTES,
        count: COUNT,
        visits: VISITS,
    })
    .unwrap();
    let writes = Arc::new(Mutex::new(Vec::new()));
    let observed = writes.clone();
    let (writer, worker) = terminal_writer::Writer::start(
        &tasks,
        terminal_writer::Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |bytes| {
            observed.lock().unwrap().push(bytes.to_vec());
            Ok(())
        },
    )
    .unwrap();
    sender
        .submit(
            terminal
                .advance(b"\x1b]4;1;#aabbcc\x07\x1b]4;1;?\x07\x1b]104;1\x07\x1b]4;1;?\x07")
                .unwrap(),
            Instant::now(),
        )
        .unwrap();
    assert_eq!(
        terminal
            .snapshot(|snapshot| snapshot.core.content().unwrap().colors[1])
            .unwrap(),
        None
    );
    let delivery = receiver.recv().await.unwrap().unwrap();
    let metadata = Arc::new(TerminalSessionMetadata::new(
        ProjectId::new(),
        "/synthetic".into(),
        "synthetic".into(),
    ));
    let mut dispatcher = Dispatcher::new(SESSION.into(), metadata);
    timeout(
        TIMEOUT,
        dispatcher.consume(&delivery, &services, &writer, &ports()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        *writes.lock().unwrap(),
        [
            b"\x1b]4;1;rgb:aaaa/bbbb/cccc\x07".to_vec(),
            b"\x1b]4;1;rgb:0101/0202/0303\x07".to_vec()
        ]
    );
    drop(delivery);
    writer.close();
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn dispatch는_cwd_명령시간_agent_관찰시각과_native_query_순서를_보존한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let events = Arc::new(Sink::default());
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-dispatch-{}", ProjectId::new())),
    ));
    let services = services(state, tasks.clone(), events.clone());
    assert_eq!(
        services
            .agents
            .classify_session_state(SESSION, AGENT)
            .activity,
        AgentActivity::Unknown
    );
    let metadata = Arc::new(TerminalSessionMetadata::new(
        ProjectId::new(),
        "/synthetic/start".into(),
        "synthetic".into(),
    ));
    let mut dispatcher = Dispatcher::new(SESSION.into(), metadata.clone());
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let (sender, mut receiver) = terminal_frames::channel(terminal_frames::Limits {
        bytes: BYTES,
        count: COUNT,
        visits: VISITS,
    })
    .unwrap();
    let writes = Arc::new(Mutex::new(Vec::new()));
    let captured = writes.clone();
    let (writer, worker) = terminal_writer::Writer::start(
        &tasks,
        terminal_writer::Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |bytes| {
            captured.lock().unwrap().push(bytes.to_vec());
            Ok(())
        },
    )
    .unwrap();
    let observed_at = Instant::now() - OLD_OUTPUT;
    sender.submit(terminal.advance(b"\x1b]7;/synthetic/one\x07\x1b]7;/synthetic/two\x07\x1b]133;C\x07\x1b]9;ready\x07\x1b]4;1;?\x07\x1b[14t\x07").unwrap(), observed_at).unwrap();
    let first = receiver.recv().await.unwrap().unwrap();
    let effects = ports();
    timeout(
        TIMEOUT,
        dispatcher.consume(&first, &services, &writer, &effects),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(metadata.cwd(), "/synthetic/two");
    assert_eq!(
        services
            .agents
            .classify_session_state(SESSION, AGENT)
            .activity,
        AgentActivity::Idle
    );
    assert_eq!(
        *writes.lock().unwrap(),
        [
            b"\x1b]4;1;rgb:0101/0202/0303\x07".to_vec(),
            b"\x1b[4;64;160t".to_vec()
        ]
    );
    drop(first);

    sender
        .submit(
            terminal
                .advance(b"\x1b]7;/synthetic/three\x07\x1b]133;D;7\x07\x1b]2;finished\x07")
                .unwrap(),
            observed_at + Duration::from_millis(COMMAND_MS),
        )
        .unwrap();
    let second = receiver.recv().await.unwrap().unwrap();
    timeout(
        TIMEOUT,
        dispatcher.consume(&second, &services, &writer, &effects),
    )
    .await
    .unwrap()
    .unwrap();
    {
        let published = events.0.lock().unwrap();
        assert_eq!(published.len(), 3);
        assert!(
            matches!(&published[0], AppEvent::TerminalCwdChanged { cwd, .. } if cwd == "/synthetic/two")
        );
        assert!(
            matches!(&published[1], AppEvent::TerminalCwdChanged { cwd, .. } if cwd == "/synthetic/three")
        );
        assert!(
            matches!(&published[2], AppEvent::TerminalCommandFinished { cwd: Some(cwd), exit_code: Some(7), duration_ms, .. } if cwd == "/synthetic/three" && u64::from(*duration_ms) == COMMAND_MS)
        );
    }
    assert!(
        dispatcher
            .consume(&second, &services, &writer, &effects)
            .await
            .is_err()
    );
    assert_eq!(events.0.lock().unwrap().len(), 3);
    writer.close();
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn renderer_관찰은_모든_query를_쓰지_않고_metadata_timer_observer를_보존한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let events = Arc::new(Sink::default());
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-renderer-observe-{}", ProjectId::new())),
    ));
    let services = services(state, tasks.clone(), events.clone());
    assert_eq!(
        services
            .agents
            .classify_session_state(SESSION, AGENT)
            .activity,
        AgentActivity::Unknown
    );
    let metadata = Arc::new(TerminalSessionMetadata::new(
        ProjectId::new(),
        "/synthetic/start".into(),
        "synthetic".into(),
    ));
    let mut dispatcher = Dispatcher::new(SESSION.into(), metadata.clone());
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let (sender, mut receiver) = terminal_frames::channel(terminal_frames::Limits {
        bytes: BYTES,
        count: COUNT,
        visits: VISITS,
    })
    .unwrap();
    let writes = Arc::new(Mutex::new(Vec::new()));
    let captured = writes.clone();
    let (writer, worker) = terminal_writer::Writer::start(
        &tasks,
        terminal_writer::Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |bytes| {
            captured.lock().unwrap().push(bytes.to_vec());
            Ok(())
        },
    )
    .unwrap();
    let updates = Arc::new(AtomicUsize::new(0));
    let updated = updates.clone();
    let streams = Arc::new(Mutex::new(Vec::new()));
    let stream = streams.clone();
    let bells = Arc::new(AtomicUsize::new(0));
    let bell = bells.clone();
    let effects = ObservePorts {
        command_colors: Default::default(),
        updated: Arc::new(move || {
            updated.fetch_add(1, Ordering::AcqRel);
        }),
        event: Arc::new(move |event| {
            assert!(matches!(event, TerminalEvent::Bell));
            bell.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }),
        stream: Arc::new(move |event| {
            stream.lock().unwrap().push(event.clone());
            Ok(())
        }),
    };
    let observed_at = Instant::now() - OLD_OUTPUT;
    sender.submit(terminal.advance(b"\x1b]7;/synthetic/two\x07\x1b]133;C\x07\x1b]9;ready\x07\x1b[5n\x1b]4;1;?\x07\x1b[14t\x07").unwrap(), observed_at).unwrap();
    let first = receiver.recv().await.unwrap().unwrap();
    assert!(first.frame().outcome.effects.iter().any(|effect| matches!(
        effect,
        taide_native_terminal::Effect::Terminal(TerminalEvent::PtyWrite(_))
    )));
    assert!(first.frame().outcome.effects.iter().any(|effect| matches!(
        effect,
        taide_native_terminal::Effect::Terminal(TerminalEvent::NativeColorRequest(..))
    )));
    assert!(first.frame().outcome.effects.iter().any(|effect| matches!(
        effect,
        taide_native_terminal::Effect::Terminal(TerminalEvent::NativeTextAreaSizeRequest(..))
    )));
    timeout(
        TIMEOUT,
        dispatcher.consume_observed(&first, &services, &writer, &effects),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(metadata.cwd(), "/synthetic/two");
    assert_eq!(
        services
            .agents
            .classify_session_state(SESSION, AGENT)
            .activity,
        AgentActivity::Idle
    );
    drop(first);
    sender
        .submit(
            terminal
                .advance(b"\x1b]7;/synthetic/three\x07\x1b]133;D;7\x07\x1b]2;finished\x07")
                .unwrap(),
            observed_at + Duration::from_millis(COMMAND_MS),
        )
        .unwrap();
    let second = receiver.recv().await.unwrap().unwrap();
    timeout(
        TIMEOUT,
        dispatcher.consume_observed(&second, &services, &writer, &effects),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        dispatcher
            .consume_observed(&second, &services, &writer, &effects)
            .await
            .is_err()
    );
    writer.close();
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(writes.lock().unwrap().is_empty());
    assert_eq!(updates.load(Ordering::Acquire), 2);
    assert_eq!(bells.load(Ordering::Acquire), 1);
    assert!(
        streams
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, ScanEvent::Title(title) if title == "finished"))
    );
    let published = events.0.lock().unwrap();
    assert_eq!(published.len(), 3);
    assert!(
        matches!(&published[2], AppEvent::TerminalCommandFinished { cwd: Some(cwd), exit_code: Some(7), duration_ms, .. } if cwd == "/synthetic/three" && u64::from(*duration_ms) == COMMAND_MS)
    );
}

#[tokio::test]
async fn query_write_실패는_partial_effect를_재실행하지_않는다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let events = Arc::new(Sink::default());
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-dispatch-{}", ProjectId::new())),
    ));
    let services = services(state, tasks.clone(), events.clone());
    let metadata = Arc::new(TerminalSessionMetadata::new(
        ProjectId::new(),
        "/synthetic/start".into(),
        "synthetic".into(),
    ));
    let mut dispatcher = Dispatcher::new(SESSION.into(), metadata);
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let (sender, mut receiver) = terminal_frames::channel(terminal_frames::Limits {
        bytes: BYTES,
        count: COUNT,
        visits: VISITS,
    })
    .unwrap();
    let attempts = Arc::new(Mutex::new(0));
    let captured = attempts.clone();
    let (writer, worker) = terminal_writer::Writer::start(
        &tasks,
        terminal_writer::Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |_| {
            *captured.lock().unwrap() += 1;
            Err(AppError::Internal("synthetic query IO failure".into()))
        },
    )
    .unwrap();
    sender
        .submit(
            terminal
                .advance(b"\x1b]7;/synthetic/changed\x07\x1b]4;1;?\x07")
                .unwrap(),
            Instant::now(),
        )
        .unwrap();
    let first = receiver.recv().await.unwrap().unwrap();
    assert!(first.frame().outcome.effects.iter().any(|effect| matches!(
        effect,
        taide_native_terminal::Effect::Stream(ScanEvent::Cwd(_))
    )));
    let effects = ports();
    assert!(
        timeout(
            TIMEOUT,
            dispatcher.consume(&first, &services, &writer, &effects)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        dispatcher
            .consume(&first, &services, &writer, &effects)
            .await
            .is_err()
    );
    assert_eq!(events.0.lock().unwrap().len(), 1);
    assert_eq!(*attempts.lock().unwrap(), 1);
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}
