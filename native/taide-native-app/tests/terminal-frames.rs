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

#[cfg(unix)]
#[tokio::test]
async fn 실제_pty의_포화는_native_core를_실패로_닫고_writer와_worker를_회수한다() {
    use std::io::Write;
    use taide_infra::{pty::PtySpawnConfig, terminal_scan::ScanEvent};
    use taide_native_app::terminal_writer;
    use taide_native_terminal::{
        Effect,
        input::{InputAction, NativeInput},
        session::{Failure, Phase},
    };
    use taide_runtime::TaskSupervisor;

    const COLUMNS: u16 = 80;
    const ROWS: u16 = 24;
    const HISTORY: usize = 128;
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
    let (sender, mut receiver) = terminal_frames::channel(Limits {
        count: 1,
        ..limits()
    })
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
    drop(first);
    assert!(matches!(receiver.recv().await, Err(Error::Capacity)));
    writer.close();
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}
