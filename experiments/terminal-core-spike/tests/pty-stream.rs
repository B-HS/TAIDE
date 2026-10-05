#![cfg(unix)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use taide_infra::pty::{PtySpawnConfig, spawn};
use taide_infra::shell_integration::CommandMarker;
use taide_infra::terminal_scan::ScanEvent;
use taide_terminal_core_spike::osc_effects::OscEffectProbe;
use taide_terminal_core_spike::{COLUMNS, HISTORY_LIMIT, SCREEN_LINES};
use tokio::sync::oneshot;
use tokio::time::{Instant, timeout_at};

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_CAPTURE_BYTES: usize = 64 * 1024;
const OUTPUT_ROWS: usize = 1_000;
const INITIAL_TEXT: &str = "한𐐀e\u{301}\n";
const READY_TITLE: &str = "fixture-ready";
const FINISHED_TITLE: &str = "fixture-finished";

#[derive(Default)]
struct Observed {
    terminal: OscEffectProbe,
    text: String,
    events: Vec<ScanEvent>,
    sequence: usize,
    failure: Option<&'static str>,
}

#[tokio::test]
async fn 실제_pty의_snapshot_이후_live_출력은_한_parser에서_보존되고_종료는_worker까지_join한다() {
    let observed = Arc::new(Mutex::new(Observed::default()));
    let sink = observed.clone();
    let (ready, started) = oneshot::channel();
    let ready = Mutex::new(Some(ready));
    let (exited, exit) = oneshot::channel();
    let deadline = Instant::now() + TEST_TIMEOUT;
    let session = spawn(
        PtySpawnConfig {
            shell: Some(env!("CARGO_BIN_EXE_pty-fixture").into()),
            cwd: env!("CARGO_MANIFEST_DIR").into(),
            cols: u16::try_from(COLUMNS).unwrap(),
            rows: u16::try_from(SCREEN_LINES).unwrap(),
            extra_env: Vec::new(),
        },
        move |bytes| {
            let mut observed = sink.lock().unwrap();
            let outcome = match observed.terminal.advance_outcome(bytes) {
                Ok(outcome) => outcome,
                Err(_) => {
                    observed.failure = Some("native effect overflow");
                    return;
                }
            };
            if observed.text.len() + outcome.text.len() > MAX_CAPTURE_BYTES {
                observed.failure = Some("fixture capture capacity exceeded");
                return;
            }
            observed.sequence += 1;
            observed.text.push_str(&outcome.text);
            let is_ready = outcome
                .events
                .iter()
                .any(|event| matches!(event,ScanEvent::Title(title) if title == READY_TITLE));
            observed.events.extend(outcome.events);
            if is_ready && let Some(ready) = ready.lock().unwrap().take() {
                ready.send(()).ok();
            }
        },
        move |status| {
            exited.send(status).ok();
        },
    )
    .unwrap();
    let completion = session.completion_handle();
    timeout_at(deadline, started).await.unwrap().unwrap();
    let (snapshot, checkpoint) = {
        let observed = observed.lock().unwrap();
        assert!(observed.failure.is_none());
        assert_eq!(observed.text, INITIAL_TEXT);
        (observed.terminal.snapshot(), observed.sequence)
    };
    assert_eq!(snapshot.lines[0], INITIAL_TEXT.trim_end());
    assert_eq!(snapshot.effects.clipboard_requests, 0);
    session.write(b"continue\n").unwrap();
    assert_eq!(timeout_at(deadline, exit).await.unwrap().unwrap(), Some(0));
    timeout_at(deadline, completion.wait_for_completion())
        .await
        .unwrap()
        .unwrap();
    assert!(completion.is_finished());
    let observed = observed.lock().unwrap();
    assert!(observed.failure.is_none());
    assert!(observed.sequence > checkpoint);
    let expected = format!(
        "{INITIAL_TEXT}{}",
        (0..OUTPUT_ROWS)
            .map(|row| format!("row-{row} 한e\u{301}\n"))
            .collect::<String>()
    );
    assert_eq!(observed.text, expected);
    assert_eq!(
        observed.events,
        [
            ScanEvent::Title(READY_TITLE.into()),
            ScanEvent::CommandMarker(CommandMarker::OutputStart),
            ScanEvent::Title(FINISHED_TITLE.into())
        ]
    );
    let final_snapshot = observed.terminal.snapshot();
    assert_eq!(final_snapshot.history.len(), HISTORY_LIMIT);
    assert_eq!(
        final_snapshot.lines[SCREEN_LINES - 2],
        format!("row-{} 한e\u{301}", OUTPUT_ROWS - 1)
    );
    assert_eq!(final_snapshot.effects.clipboard_requests, 0);
}
