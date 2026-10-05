#![cfg(unix)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use alacritty_terminal::index::{Column, Line};
use taide_infra::pty::PtySpawnConfig;
use taide_infra::terminal_scan::ScanEvent;
use taide_native_terminal::input::{InputAction, InputError, NativeInput};
use taide_native_terminal::{
    Effect, Limits, Size,
    session::{Failure, Phase, SharedTerminal},
};
use tokio::sync::oneshot;
use tokio::time::timeout;

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const HISTORY: usize = 128;
const OUTPUT_ROWS: usize = 1000;
const TIMEOUT: Duration = Duration::from_secs(3);
const CAPTURE_BYTES: usize = 64 * 1024;
const RESIZED_COLUMNS: u16 = 96;
const RESIZED_ROWS: u16 = 32;

fn terminal() -> SharedTerminal {
    SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap()
}

fn config() -> PtySpawnConfig {
    PtySpawnConfig {
        shell: Some(env!("CARGO_BIN_EXE_native-session-fixture").into()),
        cwd: env!("CARGO_MANIFEST_DIR").into(),
        cols: COLUMNS,
        rows: ROWS,
        extra_env: Vec::new(),
    }
}

#[test]
fn focus_query는_활성화마다_현재_상태를_보고하고_epoch를_보존한다() {
    let terminal = terminal();
    let epoch = terminal
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let reports = |bytes: &[u8]| {
        terminal
            .advance(bytes)
            .unwrap()
            .outcome
            .effects
            .into_iter()
            .filter_map(|effect| match effect {
                Effect::Terminal(taide_native_terminal::TerminalEvent::PtyWrite(text)) => {
                    Some(text)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(reports(b"\x1b[?1004h"), ["\x1b[O"]);
    assert_eq!(
        terminal
            .encode_input(NativeInput::Focus(true), CAPTURE_BYTES)
            .unwrap(),
        InputAction::Write(b"\x1b[I".to_vec()),
    );
    assert_eq!(reports(b"\x1b[?1004h\x1b[?1004h"), ["\x1b[I", "\x1b[I"]);
    assert!(reports(b"\x1b[?1004l").is_empty());
    assert_eq!(
        terminal
            .encode_input(NativeInput::Focus(false), CAPTURE_BYTES)
            .unwrap(),
        InputAction::Ignore,
    );
    assert_eq!(reports(b"\x1b[?1004h"), ["\x1b[O"]);
    assert_eq!(
        terminal
            .try_input(NativeInput::Focus(true), CAPTURE_BYTES, |_| Err::<(), _>(
                "pending"
            ))
            .unwrap(),
        Err("pending"),
    );
    assert_eq!(reports(b"\x1b[?1004h"), ["\x1b[I"]);
    assert_eq!(
        terminal
            .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        epoch
    );
}

#[test]
fn input_admission은_전송_승인_뒤에만_epoch를_갱신한다() {
    let terminal = terminal();
    let epoch = || {
        terminal
            .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap()
    };
    let initial = epoch();
    let result = terminal
        .try_input(NativeInput::CommittedText("a"), CAPTURE_BYTES, |action| {
            assert!(matches!(action, InputAction::Write(ref bytes) if bytes == b"a"));
            Err::<(), _>("synthetic queue full")
        })
        .unwrap();
    assert_eq!(result, Err("synthetic queue full"));
    assert_eq!(epoch(), initial);
    let accepted = terminal
        .try_input(
            NativeInput::CommittedText("a"),
            CAPTURE_BYTES,
            Ok::<_, &str>,
        )
        .unwrap()
        .unwrap();
    assert!(matches!(accepted, InputAction::Write(ref bytes) if bytes == b"a"));
    assert_eq!(epoch(), initial + 1);
    terminal.advance(b"\x1b[?1004h").unwrap();
    assert!(matches!(
        terminal
            .try_input(NativeInput::Focus(true), CAPTURE_BYTES, Ok::<_, &str>)
            .unwrap()
            .unwrap(),
        InputAction::Write(ref bytes) if bytes == b"\x1b[I"
    ));
    assert_eq!(epoch(), initial + 1);
    assert!(
        terminal
            .try_input::<(), ()>(NativeInput::CommittedText("too large"), 0, |_| {
                panic!("rejected encoding reached delivery")
            })
            .is_err()
    );
    assert_eq!(epoch(), initial + 1);
    assert_eq!(
        terminal
            .try_write(true, || Err::<(), _>("pending"))
            .unwrap(),
        Err("pending")
    );
    assert_eq!(epoch(), initial + 1);
    terminal
        .try_write(true, || Ok::<_, ()>(()))
        .unwrap()
        .unwrap();
    assert_eq!(epoch(), initial + 2);
    terminal.fail_and_stop(Failure::Delivery).unwrap();
    assert_eq!(
        terminal.try_write::<(), ()>(true, || panic!("retired input reached delivery")),
        Err(InputError::Retired)
    );
}

#[test]
fn callback_sync_deadline은_만료때만_한번_flush하고_resize_실패는_첫원인을_보존한다() {
    let terminal = terminal();
    let first = terminal
        .advance(b"\x1b[?2026hlive\x1b]2;sync-title\x07")
        .unwrap();
    let deadline = terminal
        .snapshot(|state| state.core.sync_deadline())
        .unwrap()
        .unwrap();
    let before = deadline.checked_sub(Duration::from_nanos(1)).unwrap();
    assert!(
        !terminal
            .flush_sync_if_due(before, |_| panic!("early sync must not deliver"))
            .unwrap()
    );
    assert!(terminal.flush_sync_if_due(deadline, |frame| {
        assert_eq!(frame.revision, first.revision + 1);
        assert_eq!(frame.outcome.text, "live");
        assert!(frame.outcome.effects.iter().any(|effect| matches!(effect, Effect::Stream(ScanEvent::Title(title)) if title == "sync-title")));
        true
    }).unwrap());
    assert!(
        !terminal
            .flush_sync_if_due(deadline, |_| panic!("completed sync must not repeat"))
            .unwrap()
    );
    assert!(
        terminal
            .resize_with_delivery(
                Size {
                    columns: RESIZED_COLUMNS,
                    rows: RESIZED_ROWS
                },
                || Err(taide_model::error::AppError::Internal(
                    "synthetic resize failure".into()
                )),
                |_| panic!("failed resize must not deliver")
            )
            .is_err()
    );
    terminal
        .snapshot(|state| {
            assert_eq!(state.phase, Phase::Failed(Failure::Resize));
            assert!(state.core.grid().is_err());
        })
        .unwrap();
    terminal.fail_and_stop(Failure::Delivery).unwrap();
    assert_eq!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Failed(Failure::Resize)
    );
}

#[derive(Default)]
struct Capture {
    revisions: Vec<u64>,
    text: String,
    titles: Vec<String>,
}

#[tokio::test]
async fn 실제_pty는_단일_core_snapshot_live와_join_뒤_마지막_sync를_보존한다() {
    let terminal = terminal();
    let view = terminal.clone();
    assert_eq!(view.snapshot(|state| state.revision).unwrap(), 0);
    let captured = Arc::new(Mutex::new(Capture::default()));
    let observed = captured.clone();
    let (send_ready, ready) = oneshot::channel();
    let send_ready = Mutex::new(Some(send_ready));
    let pty = terminal
        .spawn(config(), move |frame| {
            let mut observed = observed.lock().unwrap();
            if observed.text.len() + frame.outcome.text.len() > CAPTURE_BYTES {
                return false;
            }
            observed.revisions.push(frame.revision);
            observed.text.push_str(&frame.outcome.text);
            for effect in frame.outcome.effects {
                if let Effect::Stream(ScanEvent::Title(title)) = effect {
                    if title == "native-ready"
                        && let Some(sender) = send_ready.lock().unwrap().take()
                    {
                        let _ = sender.send(());
                    }
                    observed.titles.push(title);
                }
            }
            true
        })
        .unwrap();
    let completion = pty.completion_handle();
    let stop = completion.stop_handle();
    assert!(stop.is_same_worker(&completion));
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    let checkpoint = view
        .snapshot(|snapshot| {
            assert_eq!(snapshot.phase, Phase::Running);
            let grid = snapshot.core.grid().unwrap();
            assert_eq!(grid[Line(0)][Column(0)].c, '한');
            assert_eq!(grid[Line(0)][Column(2)].c, '𐐀');
            snapshot.revision
        })
        .unwrap();
    assert!(checkpoint > 0);
    assert!(terminal.spawn(config(), |_| true).is_err());
    let InputAction::Write(input) = view
        .encode_input(NativeInput::CommittedText("continue\n"), CAPTURE_BYTES)
        .unwrap()
    else {
        panic!("fixture input was not encoded");
    };
    pty.write(&input).unwrap();
    let final_frame = timeout(TIMEOUT, terminal.finish(&completion))
        .await
        .unwrap()
        .unwrap();
    assert!(completion.is_finished());
    assert!(final_frame.revision > checkpoint);
    assert!(final_frame.outcome.text.ends_with("final-sync"));
    assert!(final_frame.outcome.effects.iter().any(|effect| matches!(effect, Effect::Stream(ScanEvent::Title(title)) if title == "native-finished")));
    view.snapshot(|snapshot| {
        assert_eq!(snapshot.phase, Phase::Exited(Some(0)));
        assert_eq!(snapshot.revision, final_frame.revision);
        assert!(snapshot.core.grid().is_ok());
    })
    .unwrap();
    assert_eq!(
        view.encode_input(NativeInput::CommittedText("late"), CAPTURE_BYTES),
        Err(InputError::Retired)
    );
    assert!(terminal.advance(b"late").is_err());
    assert!(terminal.finish(&completion).await.is_err());
    let observed = captured.lock().unwrap();
    let expected = format!(
        "한𐐀e\u{301}\n{}",
        (0..OUTPUT_ROWS)
            .map(|row| format!("row-{row} 한e\u{301}\n"))
            .collect::<String>()
    );
    assert_eq!(observed.text, expected);
    assert_eq!(observed.titles, ["native-ready"]);
    assert!(
        observed
            .revisions
            .windows(2)
            .all(|pair| pair[1] == pair[0] + 1)
    );
    assert_eq!(final_frame.revision, observed.revisions.last().unwrap() + 1);
    drop(observed);
    drop(pty);
    drop(completion);
    stop.kill().unwrap();
}

#[tokio::test]
async fn delivery_거절은_pty를_중단하고_failed_core를_종료_성공으로_바꾸지_않는다() {
    let terminal = terminal();
    let pty = terminal.spawn(config(), |_| false).unwrap();
    let completion = pty.completion_handle();
    timeout(TIMEOUT, completion.wait_for_completion())
        .await
        .unwrap()
        .unwrap();
    assert!(completion.is_finished());
    terminal
        .snapshot(|snapshot| {
            assert_eq!(snapshot.phase, Phase::Failed(Failure::Delivery));
            assert!(snapshot.core.grid().is_err());
        })
        .unwrap();
    assert!(terminal.finish(&completion).await.is_err());
    assert!(terminal.advance(b"partial-success").is_err());
}

#[tokio::test]
async fn callback_panic은_실제_pty를_중단하고_다른_completion은_거절한다() {
    let active = terminal();
    let (send_ready, ready) = oneshot::channel();
    let send_ready = Mutex::new(Some(send_ready));
    let active_pty = active
        .spawn(config(), move |_| {
            if let Some(sender) = send_ready.lock().unwrap().take() {
                let _ = sender.send(());
            }
            true
        })
        .unwrap();
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    let panicked = terminal();
    let panic_pty = panicked
        .spawn(config(), |_| panic!("synthetic delivery panic"))
        .unwrap();
    let completion = panic_pty.completion_handle();
    assert!(
        timeout(TIMEOUT, completion.wait_for_completion())
            .await
            .unwrap()
            .is_err()
    );
    assert!(!completion.is_finished());
    assert!(active.finish(&completion).await.is_err());
    assert_eq!(
        active.snapshot(|snapshot| snapshot.phase).unwrap(),
        Phase::Running
    );
    assert!(panicked.finish(&completion).await.is_err());
    assert_eq!(
        panicked.snapshot(|snapshot| snapshot.phase).unwrap(),
        Phase::Failed(Failure::Delivery)
    );
    active_pty.kill().unwrap();
    let active_completion = active_pty.completion_handle();
    timeout(TIMEOUT, active.finish(&active_completion))
        .await
        .unwrap()
        .unwrap();
    assert!(active_completion.is_finished());
}
