#![cfg(unix)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use taide_infra::pty::{PtySpawnConfig, spawn};
use taide_infra::terminal_scan::ScanEvent;
use taide_terminal_core_spike::input::{InputAction, Key, Modifiers, NativeInput};
use taide_terminal_core_spike::osc_effects::OscEffectProbe;
use taide_terminal_core_spike::{COLUMNS, SCREEN_LINES};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Instant, timeout_at};

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_INPUT_BYTES: usize = 64 * 1024;
const EVENT_CAPACITY: usize = 8;
const COMMIT: &str = "한e\u{301}𐐀";
const PASTE: &str = "한\r\n日\n";
const PHASES: &[&str] = &["normal", "application", "reset"];

#[derive(Default)]
struct Observed {
    terminal: OscEffectProbe,
    events: Vec<String>,
    failure: Option<&'static str>,
}

#[tokio::test]
async fn input_roundtrip의_실제_pty는_live_mode의_정확한_byte를_수신하고_한_session에서_종료한다() {
    let observed = Arc::new(Mutex::new(Observed::default()));
    let sink = observed.clone();
    let (titles, mut received_titles) = mpsc::channel(EVENT_CAPACITY);
    let (exited, exit) = oneshot::channel();
    let deadline = Instant::now() + TEST_TIMEOUT;
    let session = spawn(
        PtySpawnConfig {
            shell: Some(env!("CARGO_BIN_EXE_pty-input-fixture").into()),
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
                    observed.failure = Some("fixture parser capacity exceeded");
                    return;
                }
            };
            for event in outcome.events {
                if let ScanEvent::Title(title) = event {
                    observed.events.push(title.clone());
                    if titles.try_send(title).is_err() {
                        observed.failure = Some("fixture event channel capacity exceeded");
                    }
                }
            }
        },
        move |status| {
            exited.send(status).ok();
        },
    )
    .unwrap();
    let completion = session.completion_handle();
    let plain = Modifiers::default();
    let shift = Modifiers {
        shift: true,
        ..plain
    };
    let control = Modifiers {
        control: true,
        ..plain
    };
    let alt = Modifiers { alt: true, ..plain };
    for phase in PHASES {
        let ready = timeout_at(deadline, received_titles.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ready, format!("input-{phase}-ready"));
        let operations = match *phase {
            "normal" => vec![
                NativeInput::Key {
                    key: Key::ArrowUp,
                    modifiers: plain,
                },
                NativeInput::Key {
                    key: Key::Character('c'),
                    modifiers: control,
                },
                NativeInput::Key {
                    key: Key::Enter,
                    modifiers: plain,
                },
                NativeInput::Key {
                    key: Key::Enter,
                    modifiers: shift,
                },
                NativeInput::Preedit(COMMIT),
                NativeInput::CommittedText(COMMIT),
                NativeInput::Paste(PASTE),
                NativeInput::Focus(true),
            ],
            "application" => vec![
                NativeInput::Key {
                    key: Key::ArrowUp,
                    modifiers: plain,
                },
                NativeInput::Key {
                    key: Key::Backspace,
                    modifiers: alt,
                },
                NativeInput::Focus(true),
                NativeInput::Focus(false),
                NativeInput::Paste(PASTE),
            ],
            "reset" => vec![
                NativeInput::Key {
                    key: Key::ArrowUp,
                    modifiers: plain,
                },
                NativeInput::Paste(PASTE),
                NativeInput::Focus(false),
            ],
            _ => unreachable!("fixed synthetic phase"),
        };
        let bytes = {
            let observed = observed.lock().unwrap();
            assert!(observed.failure.is_none());
            let mut bytes = Vec::new();
            for operation in operations {
                match observed
                    .terminal
                    .encode_input(operation, MAX_INPUT_BYTES)
                    .unwrap()
                {
                    InputAction::Write(encoded) => bytes.extend(encoded),
                    InputAction::Ignore => {}
                    _ => panic!("fixture unexpectedly requested a local UI action"),
                }
            }
            bytes
        };
        session.write(&bytes).unwrap();
        let receipt = timeout_at(deadline, received_titles.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(receipt, format!("input-{phase}-received"));
    }
    assert_eq!(
        timeout_at(deadline, received_titles.recv())
            .await
            .unwrap()
            .unwrap(),
        "input-finished"
    );
    assert_eq!(timeout_at(deadline, exit).await.unwrap().unwrap(), Some(0));
    timeout_at(deadline, completion.wait_for_completion())
        .await
        .unwrap()
        .unwrap();
    assert!(completion.is_finished());
    let observed = observed.lock().unwrap();
    assert!(observed.failure.is_none());
    let mut expected = PHASES
        .iter()
        .flat_map(|phase| {
            [
                format!("input-{phase}-ready"),
                format!("input-{phase}-received"),
            ]
        })
        .collect::<Vec<_>>();
    expected.push("input-finished".into());
    assert_eq!(observed.events, expected);
    assert_eq!(observed.terminal.snapshot().effects.clipboard_requests, 0);
}
