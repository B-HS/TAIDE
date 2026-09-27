#[cfg(unix)]
mod unix_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};
    use std::time::{Duration, Instant};

    use taide_infra::pty::PtySpawnConfig;
    use taide_terminal::runtime::spawn_terminal_session;
    use taide_terminal::session::{TerminalSessionOutput, TERMINAL_REPLAY_PREAMBLE};

    const TEST_COLS: u16 = 80;
    const TEST_ROWS: u16 = 24;
    const TEST_SCROLLBACK_BYTES: usize = 64 * 1024;
    const CALLBACK_TIMEOUT_SECS: u64 = 3;

    #[test]
    fn 실제_pty_출력은_기록과_스캔을_거쳐_콜백에_도달한다() {
        let output = Arc::new(TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES));
        let observed_bytes = Arc::new(AtomicUsize::new(0));
        let counter = observed_bytes.clone();
        let scan_counter = observed_bytes.clone();
        let scan_output = output.clone();
        let (scan_sender, scan_receiver) = mpsc::channel();
        let (exit_sender, exit_receiver) = mpsc::channel();

        let session = spawn_terminal_session(
            PtySpawnConfig {
                shell: Some("/bin/sh".to_string()),
                cwd: std::env::temp_dir().to_string_lossy().to_string(),
                cols: TEST_COLS,
                rows: TEST_ROWS,
                extra_env: vec![("ENV".to_string(), String::new()), ("BASH_ENV".to_string(), String::new())],
            },
            output.clone(),
            move |bytes| {
                counter.fetch_add(bytes.len(), Ordering::SeqCst);
            },
            move |outcome| {
                let replay = scan_output.attach(|_| true);
                scan_output.detach(replay.subscription_id);
                scan_sender
                    .send((outcome.clone(), scan_counter.load(Ordering::SeqCst), replay.replay_bytes))
                    .ok();
            },
            move |code| {
                exit_sender.send(code).ok();
            },
        )
        .expect("테스트 PTY 생성");

        session.write(b"printf '\\033]7;/tmp\\007'; exit\n").expect("명령 전송");

        let deadline = Instant::now() + Duration::from_secs(CALLBACK_TIMEOUT_SECS);
        let mut found_cwd = false;
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let (outcome, counted_bytes, replay_bytes) = scan_receiver.recv_timeout(remaining).expect("출력 스캔 수신");
            if outcome.latest_cwd() == Some("/tmp") {
                assert!(counted_bytes > 0);
                assert!(replay_bytes as usize > TERMINAL_REPLAY_PREAMBLE.len());
                found_cwd = true;
                break;
            }
        }

        assert!(found_cwd);
        assert!(observed_bytes.load(Ordering::SeqCst) > 0);
        let attached = output.attach(|_| true);
        assert!(attached.replay_bytes as usize > TERMINAL_REPLAY_PREAMBLE.len());
        assert_eq!(exit_receiver.recv_timeout(Duration::from_secs(CALLBACK_TIMEOUT_SECS)), Ok(Some(0)));
    }
}
