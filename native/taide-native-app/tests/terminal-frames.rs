use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use taide_native_app::terminal_frames::{self, Error, Limits};
use taide_native_retained::measure;
use taide_native_terminal::{
    Outcome, Size,
    session::{Frame, SharedTerminal},
};
use tokio::time::timeout;

const BYTES: usize = 64 * 1024;
const VISITS: usize = 1024;
const COUNT: usize = 2;
const TIMEOUT: Duration = Duration::from_secs(3);
const APPLICATION_BYTES: usize = 4 * 1024 * 1024;
const APPLICATION_COUNT: usize = 64;
const APPLICATION_VISITS: usize = 1_000_000;
#[cfg(unix)]
const PAUSE_PROBE: Duration = Duration::from_millis(200);

fn limits() -> Limits {
    Limits {
        bytes: BYTES,
        count: COUNT,
        visits: VISITS,
    }
}

fn frame(revision: u64, capacity: usize) -> Frame {
    let mut text = String::with_capacity(capacity);
    text.push('x');
    Frame {
        revision,
        outcome: Outcome {
            effects: Vec::new(),
            text,
            overlap: String::new(),
        },
    }
}

#[tokio::test]
async fn actual_core_frame은_borrowed_delivery를_유지한_동안_count를_반납하지_않는다() {
    let terminal = SharedTerminal::new(
        Size {
            columns: 20,
            rows: 3,
        },
        10,
        Default::default(),
    )
    .unwrap();
    let (sender, mut receiver) = terminal_frames::channel(limits()).unwrap();
    let first_at = Instant::now();
    sender
        .submit(
            terminal
                .advance("한글e\u{301}\x1b]2;title\x07".as_bytes())
                .unwrap(),
            first_at,
        )
        .unwrap();
    let first = receiver.recv().await.unwrap().unwrap();
    assert_eq!(first.frame().revision, 1);
    assert!(first.frame().outcome.text.contains("한글e\u{301}"));
    assert_eq!(first.observed_at(), first_at);
    sender
        .submit(terminal.advance(b"two").unwrap(), first_at)
        .unwrap();
    assert_eq!(
        sender.submit(terminal.advance(b"three").unwrap(), first_at),
        Err(Error::Capacity)
    );
    let second = receiver.recv().await.unwrap().unwrap();
    assert_eq!(second.frame().revision, 2);
    assert_eq!(second.frame().outcome.text, "two");
    drop(first);
    drop(second);
    assert!(matches!(receiver.recv().await, Err(Error::Capacity)));
    assert_eq!(sender.submit(frame(3, 1), first_at), Err(Error::Capacity));
}

#[tokio::test]
async fn byte_quota는_received_frame과_text_capacity를_포함한다() {
    let exact_frame = frame(1, BYTES / COUNT);
    let retained = measure(
        &exact_frame,
        taide_native_retained::Limits {
            bytes: BYTES,
            visits: VISITS,
        },
    )
    .unwrap();
    let exact_bytes = retained.bytes + size_of::<terminal_frames::Delivery>() - size_of::<Frame>();
    let (sender, mut receiver) = terminal_frames::channel(Limits {
        bytes: exact_bytes,
        ..limits()
    })
    .unwrap();
    sender.submit(exact_frame, Instant::now()).unwrap();
    let first = receiver.recv().await.unwrap().unwrap();
    assert_eq!(first.frame().outcome.text, "x");
    assert_eq!(
        sender.submit(frame(2, 1), Instant::now()),
        Err(Error::Capacity)
    );
    drop(first);
    assert!(matches!(receiver.recv().await, Err(Error::Capacity)));

    let (sender, mut receiver) = terminal_frames::channel(limits()).unwrap();
    assert_eq!(
        sender.submit(frame(1, BYTES), Instant::now()),
        Err(Error::Retained(taide_native_retained::Error::ByteBudget))
    );
    assert!(matches!(
        receiver.recv().await,
        Err(Error::Retained(taide_native_retained::Error::ByteBudget))
    ));
}

#[tokio::test]
async fn 순서_실패와_close는_빈_receiver를_깨우고_첫_실패를_유지한다() {
    let (sender, mut receiver) = terminal_frames::channel(limits()).unwrap();
    let failure = async {
        tokio::task::yield_now().await;
        assert_eq!(
            sender.submit(frame(2, 1), Instant::now()),
            Err(Error::Sequence)
        );
        sender.close();
    };
    let (result, ()) = timeout(TIMEOUT, async { tokio::join!(receiver.recv(), failure) })
        .await
        .unwrap();
    assert!(matches!(result, Err(Error::Sequence)));
    assert_eq!(
        sender.submit(frame(1, 1), Instant::now()),
        Err(Error::Sequence)
    );

    let (sender, mut receiver) = terminal_frames::channel(limits()).unwrap();
    sender.submit(frame(1, 1), Instant::now()).unwrap();
    assert_eq!(
        sender.submit(frame(1, 1), Instant::now()),
        Err(Error::Sequence)
    );
    assert_eq!(receiver.recv().await.unwrap().unwrap().frame().revision, 1);
    assert!(matches!(receiver.recv().await, Err(Error::Sequence)));

    let (sender, mut receiver) = terminal_frames::channel(limits()).unwrap();
    drop(sender);
    assert!(receiver.recv().await.unwrap().is_none());
    let (sender, receiver) = terminal_frames::channel(limits()).unwrap();
    drop(receiver);
    assert_eq!(
        sender.submit(frame(1, 1), Instant::now()),
        Err(Error::Closed)
    );
    assert!(matches!(
        terminal_frames::channel(Limits {
            visits: 0,
            ..limits()
        }),
        Err(Error::InvalidLimits)
    ));
}

#[tokio::test]
async fn 수위_신호는_high에서_보류하고_low에서_재개하며_닫힘은_보류를_푼다() {
    const FLOW_COUNT: usize = 16;
    const HIGH_COUNT: u64 = (FLOW_COUNT / 2) as u64;
    const HEAVY_FRAME_BYTES: usize = terminal_frames::HIGH_WATER_BYTES / 2;
    assert_eq!(terminal_frames::HIGH_WATER_BYTES, 512 * 1024);
    assert_eq!(terminal_frames::LOW_WATER_BYTES, 64 * 1024);

    let signals = Arc::new(Mutex::new(Vec::new()));
    let observed = signals.clone();
    let (sender, mut receiver) = terminal_frames::channel_with_flow(
        Limits {
            count: FLOW_COUNT,
            ..limits()
        },
        Arc::new(move |paused| observed.lock().unwrap().push(paused)),
    )
    .unwrap();
    for revision in 1..HIGH_COUNT {
        sender.submit(frame(revision, 1), Instant::now()).unwrap();
    }
    assert!(signals.lock().unwrap().is_empty());
    sender.submit(frame(HIGH_COUNT, 1), Instant::now()).unwrap();
    sender
        .submit(frame(HIGH_COUNT + 1, 1), Instant::now())
        .unwrap();
    assert_eq!(*signals.lock().unwrap(), [true]);
    for _ in 0..HIGH_COUNT {
        drop(receiver.recv().await.unwrap().unwrap());
    }
    assert_eq!(*signals.lock().unwrap(), [true]);
    drop(receiver.recv().await.unwrap().unwrap());
    assert_eq!(*signals.lock().unwrap(), [true, false]);

    let signals = Arc::new(Mutex::new(Vec::new()));
    let observed = signals.clone();
    let (sender, mut receiver) = terminal_frames::channel_with_flow(
        Limits {
            bytes: APPLICATION_BYTES,
            count: APPLICATION_COUNT,
            visits: APPLICATION_VISITS,
        },
        Arc::new(move |paused| observed.lock().unwrap().push(paused)),
    )
    .unwrap();
    sender
        .submit(frame(1, HEAVY_FRAME_BYTES), Instant::now())
        .unwrap();
    assert!(signals.lock().unwrap().is_empty());
    sender
        .submit(frame(2, HEAVY_FRAME_BYTES), Instant::now())
        .unwrap();
    assert_eq!(*signals.lock().unwrap(), [true]);
    drop(receiver.recv().await.unwrap().unwrap());
    assert_eq!(*signals.lock().unwrap(), [true]);
    drop(receiver.recv().await.unwrap().unwrap());
    assert_eq!(*signals.lock().unwrap(), [true, false]);
    sender
        .submit(frame(3, HEAVY_FRAME_BYTES), Instant::now())
        .unwrap();
    sender
        .submit(frame(4, HEAVY_FRAME_BYTES), Instant::now())
        .unwrap();
    assert_eq!(*signals.lock().unwrap(), [true, false, true]);
    drop(receiver);
    assert_eq!(*signals.lock().unwrap(), [true, false, true, false]);
    assert_eq!(
        sender.submit(frame(5, 1), Instant::now()),
        Err(Error::Closed)
    );
    assert_eq!(*signals.lock().unwrap(), [true, false, true, false]);
}

#[cfg(unix)]
#[tokio::test]
async fn 실제_pty의_포화는_출력을_보류하고_소비_뒤_재개해_끝까지_전달한다() {
    use std::io::Write;
    use taide_infra::{pty::PtySpawnConfig, terminal_scan::ScanEvent};
    use taide_native_app::terminal_writer;
    use taide_native_terminal::{
        Effect,
        input::{InputAction, NativeInput},
        session::Phase,
    };
    use taide_runtime::TaskSupervisor;

    const COLUMNS: u16 = 80;
    const ROWS: u16 = 24;
    const HISTORY: usize = 128;
    const OUTPUT_ROWS: usize = 1000;
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let paused_terminal = terminal.clone();
    let (sender, mut receiver) = terminal_frames::channel_with_flow(
        Limits {
            count: 1,
            ..limits()
        },
        Arc::new(move |paused| paused_terminal.set_output_paused(paused)),
    )
    .unwrap();
    let output = sender.clone();
    let pty = terminal
        .spawn(
            PtySpawnConfig {
                shell: Some(env!("CARGO_BIN_EXE_native-terminal-queue-fixture").into()),
                cwd: env!("CARGO_MANIFEST_DIR").into(),
                cols: COLUMNS,
                rows: ROWS,
                extra_env: Vec::new(),
            },
            move |frame| output.submit(frame, Instant::now()).is_ok(),
        )
        .unwrap();
    let completion = pty.completion_handle();
    let first = timeout(TIMEOUT, async {
        loop {
            let delivery = receiver.recv().await.unwrap().unwrap();
            if delivery.frame().outcome.effects.iter().any(|effect| matches!(effect, Effect::Stream(ScanEvent::Title(title)) if title == "native-ready")) {
                return delivery;
            }
        }
    }).await.unwrap();
    assert_eq!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Running
    );
    let handle = pty.writer_handle();
    let (writer, worker) = terminal_writer::Writer::start(
        &tasks,
        terminal_writer::Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |bytes| {
            let mut writer = handle.lock();
            writer.write_all(bytes)?;
            writer.flush()?;
            Ok(())
        },
    )
    .unwrap();
    let InputAction::Write(input) = terminal
        .encode_input(NativeInput::CommittedText("continue\n"), BYTES)
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    timeout(TIMEOUT, writer.submit(input).unwrap().wait())
        .await
        .unwrap()
        .unwrap();
    assert!(
        timeout(PAUSE_PROBE, completion.wait_for_completion())
            .await
            .is_err()
    );
    assert!(matches!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Running | Phase::Draining(_)
    ));
    assert!(matches!(receiver.try_recv(), Ok(None)));
    drop(first);
    let mut text = String::new();
    let final_frame = timeout(TIMEOUT, async {
        let finished = terminal.finish(&completion);
        tokio::pin!(finished);
        loop {
            tokio::select! {
                frame = &mut finished => break frame,
                delivery = receiver.recv() => {
                    text.push_str(&delivery.unwrap().unwrap().frame().outcome.text);
                }
            }
        }
    })
    .await
    .unwrap()
    .unwrap();
    while let Some(delivery) = receiver.try_recv().unwrap() {
        text.push_str(&delivery.frame().outcome.text);
    }
    assert!(completion.is_finished());
    assert_eq!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    assert_eq!(
        text,
        (0..OUTPUT_ROWS)
            .map(|row| format!("row-{row} 한e\u{301}\n"))
            .collect::<String>()
    );
    assert!(final_frame.outcome.text.ends_with("final-sync"));
    writer.close();
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn 실제_pty의_과부하_출력은_high_water에서_보류되고_유실없이_끝까지_전달된다() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use taide_infra::{pty::PtySpawnConfig, terminal_scan::ScanEvent};
    use taide_native_terminal::{
        Effect,
        input::{InputAction, NativeInput},
        session::Phase,
    };
    use tokio::sync::Notify;

    const COLUMNS: u16 = 80;
    const ROWS: u16 = 24;
    const HISTORY: usize = 128;
    const FLOOD_ROWS: usize = 60_000;
    const FLOOD_TIMEOUT: Duration = Duration::from_secs(60);
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let pauses = Arc::new(AtomicUsize::new(0));
    let saturated = Arc::new(Notify::new());
    let paused_terminal = terminal.clone();
    let counted = pauses.clone();
    let signal = saturated.clone();
    let (sender, mut receiver) = terminal_frames::channel_with_flow(
        Limits {
            bytes: APPLICATION_BYTES,
            count: APPLICATION_COUNT,
            visits: APPLICATION_VISITS,
        },
        Arc::new(move |paused| {
            paused_terminal.set_output_paused(paused);
            if paused {
                counted.fetch_add(1, Ordering::SeqCst);
                signal.notify_one();
            }
        }),
    )
    .unwrap();
    let output = sender.clone();
    let pty = terminal
        .spawn(
            PtySpawnConfig {
                shell: Some(env!("CARGO_BIN_EXE_native-terminal-queue-fixture").into()),
                cwd: env!("CARGO_MANIFEST_DIR").into(),
                cols: COLUMNS,
                rows: ROWS,
                extra_env: vec![("TAIDE_NATIVE_FIXTURE_FLOOD".into(), "1".into())],
            },
            move |frame| output.submit(frame, Instant::now()).is_ok(),
        )
        .unwrap();
    let completion = pty.completion_handle();
    timeout(TIMEOUT, async {
        loop {
            let delivery = receiver.recv().await.unwrap().unwrap();
            if delivery.frame().outcome.effects.iter().any(|effect| matches!(effect, Effect::Stream(ScanEvent::Title(title)) if title == "native-ready")) {
                return;
            }
        }
    })
    .await
    .unwrap();
    let InputAction::Write(input) = terminal
        .encode_input(NativeInput::CommittedText("continue\n"), BYTES)
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    pty.write(&input).unwrap();
    timeout(FLOOD_TIMEOUT, saturated.notified()).await.unwrap();
    assert!(
        timeout(PAUSE_PROBE, completion.wait_for_completion())
            .await
            .is_err()
    );
    assert!(matches!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Running | Phase::Draining(_)
    ));
    let mut text = String::new();
    timeout(FLOOD_TIMEOUT, async {
        let finished = terminal.finish(&completion);
        tokio::pin!(finished);
        loop {
            tokio::select! {
                frame = &mut finished => break frame,
                delivery = receiver.recv() => {
                    text.push_str(&delivery.unwrap().unwrap().frame().outcome.text);
                }
            }
        }
    })
    .await
    .unwrap()
    .unwrap();
    while let Some(delivery) = receiver.try_recv().unwrap() {
        text.push_str(&delivery.frame().outcome.text);
    }
    assert_eq!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    assert!(pauses.load(Ordering::SeqCst) > 0);
    let expected = (0..FLOOD_ROWS)
        .map(|row| format!("row-{row} 한e\u{301}\n"))
        .collect::<String>();
    assert!(
        text == expected,
        "flood output was lost or reordered: {} of {} bytes",
        text.len(),
        expected.len()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn 보류된_실제_pty는_수신자_소멸로_풀려_실패로_닫히고_worker를_회수한다() {
    use taide_infra::{pty::PtySpawnConfig, terminal_scan::ScanEvent};
    use taide_native_terminal::{
        Effect,
        input::{InputAction, NativeInput},
        session::{Failure, Phase},
    };

    const COLUMNS: u16 = 80;
    const ROWS: u16 = 24;
    const HISTORY: usize = 128;
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let paused_terminal = terminal.clone();
    let (sender, mut receiver) = terminal_frames::channel_with_flow(
        Limits {
            count: 1,
            ..limits()
        },
        Arc::new(move |paused| paused_terminal.set_output_paused(paused)),
    )
    .unwrap();
    let output = sender.clone();
    let pty = terminal
        .spawn(
            PtySpawnConfig {
                shell: Some(env!("CARGO_BIN_EXE_native-terminal-queue-fixture").into()),
                cwd: env!("CARGO_MANIFEST_DIR").into(),
                cols: COLUMNS,
                rows: ROWS,
                extra_env: Vec::new(),
            },
            move |frame| output.submit(frame, Instant::now()).is_ok(),
        )
        .unwrap();
    let completion = pty.completion_handle();
    let first = timeout(TIMEOUT, async {
        loop {
            let delivery = receiver.recv().await.unwrap().unwrap();
            if delivery.frame().outcome.effects.iter().any(|effect| matches!(effect, Effect::Stream(ScanEvent::Title(title)) if title == "native-ready")) {
                return delivery;
            }
        }
    })
    .await
    .unwrap();
    let InputAction::Write(input) = terminal
        .encode_input(NativeInput::CommittedText("continue\n"), BYTES)
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    pty.write(&input).unwrap();
    assert!(
        timeout(PAUSE_PROBE, completion.wait_for_completion())
            .await
            .is_err()
    );
    drop(receiver);
    timeout(TIMEOUT, completion.wait_for_completion())
        .await
        .unwrap()
        .unwrap();
    assert!(completion.is_finished());
    assert_eq!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Failed(Failure::Delivery)
    );
    assert!(terminal.finish(&completion).await.is_err());
    assert_eq!(
        sender.submit(frame(u64::MAX, 1), Instant::now()),
        Err(Error::Closed)
    );
    drop(first);
}
