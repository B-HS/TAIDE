use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use parking_lot::Mutex;
use taide_terminal::session::{TerminalSessionOutput, TERMINAL_REPLAY_PREAMBLE};

const TEST_SCROLLBACK_BYTES: usize = 64 * 1024;
const ATTACH_BLOCK_PROBE_MS: u64 = 50;

#[test]
fn 재생과_실시간_출력은_각_바이트를_한_번씩_전달하고_detach는_해당_구독만_제거한다() {
    let output = TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES);
    output.append_and_broadcast(b"before");

    let received = Arc::new(Mutex::new(Vec::new()));
    let sink_received = received.clone();
    let attached = output.attach(move |bytes| {
        sink_received.lock().push(bytes.to_vec());
        true
    });

    assert_eq!(attached.subscription_id, 0);
    assert_eq!(attached.replay_bytes as usize, TERMINAL_REPLAY_PREAMBLE.len() + b"before".len());
    output.append_and_broadcast(b"after");
    assert_eq!(received.lock().concat(), [TERMINAL_REPLAY_PREAMBLE, b"beforeafter"].concat());

    output.detach(attached.subscription_id);
    output.append_and_broadcast(b"ignored");
    assert_eq!(received.lock().concat(), [TERMINAL_REPLAY_PREAMBLE, b"beforeafter"].concat());
}

#[test]
fn 실패한_구독자는_첫_브로드캐스트에서_정리하고_나머지는_계속_받는다() {
    let output = TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES);
    let failed_calls = Arc::new(AtomicUsize::new(0));
    let sink_failed_calls = failed_calls.clone();
    output.attach(move |_| {
        sink_failed_calls.fetch_add(1, Ordering::SeqCst);
        false
    });

    let received = Arc::new(Mutex::new(Vec::new()));
    let sink_received = received.clone();
    output.attach(move |bytes| {
        sink_received.lock().push(bytes.to_vec());
        true
    });

    output.append_and_broadcast(b"first");
    let calls_after_first_broadcast = failed_calls.load(Ordering::SeqCst);
    output.append_and_broadcast(b"second");

    assert_eq!(failed_calls.load(Ordering::SeqCst), calls_after_first_broadcast);
    assert_eq!(received.lock().concat(), [TERMINAL_REPLAY_PREAMBLE, b"firstsecond"].concat());
}

#[test]
fn attach_재생과_구독_등록_사이에_reader_출력이_끼어들지_않는다() {
    let output = Arc::new(TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES));
    output.append_and_broadcast(b"before");

    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let received = Arc::new(Mutex::new(Vec::new()));
    let attach_output = output.clone();
    let sink_entered = entered.clone();
    let sink_release = release.clone();
    let sink_received = received.clone();
    let attaching = std::thread::spawn(move || {
        attach_output.attach(move |bytes| {
            if bytes == TERMINAL_REPLAY_PREAMBLE {
                sink_entered.wait();
                sink_release.wait();
            }
            sink_received.lock().push(bytes.to_vec());
            true
        })
    });

    entered.wait();
    let appended = Arc::new(AtomicBool::new(false));
    let writer_output = output.clone();
    let writer_appended = appended.clone();
    let writing = std::thread::spawn(move || {
        writer_output.append_and_broadcast(b"during");
        writer_appended.store(true, Ordering::SeqCst);
    });

    std::thread::sleep(std::time::Duration::from_millis(ATTACH_BLOCK_PROBE_MS));
    assert!(!appended.load(Ordering::SeqCst));
    release.wait();
    attaching.join().expect("attach 스레드 종료");
    writing.join().expect("출력 스레드 종료");
    assert_eq!(received.lock().concat(), [TERMINAL_REPLAY_PREAMBLE, b"beforeduring"].concat());
}

#[test]
fn 스크롤백이_여러_구간으로_나뉘어도_재생_바이트_수는_실제_전송량과_일치한다() {
    const RING_CAPACITY: usize = 1024;
    const CHUNK: &[u8] = b"0123456789";
    const APPEND_COUNT: usize = 1024;

    let output = TerminalSessionOutput::new(RING_CAPACITY);
    for _ in 0..APPEND_COUNT {
        output.append_and_broadcast(CHUNK);
    }

    let received = Arc::new(Mutex::new(Vec::new()));
    let sink_received = received.clone();
    let attached = output.attach(move |bytes| {
        sink_received.lock().push(bytes.to_vec());
        true
    });

    assert_eq!(attached.replay_bytes as usize, received.lock().concat().len());
}

#[test]
fn 빈_스크롤백은_프리앰블만_재생한다() {
    let output = TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES);
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink_received = received.clone();
    let attached = output.attach(move |bytes| {
        sink_received.lock().push(bytes.to_vec());
        true
    });

    assert_eq!(received.lock().concat(), TERMINAL_REPLAY_PREAMBLE);
    assert_eq!(attached.replay_bytes as usize, TERMINAL_REPLAY_PREAMBLE.len());
}

#[test]
fn 스트리밍_중_attach해도_전체_출력이_중복과_유실_없이_재구성된다() {
    const STREAM_CHUNKS: usize = 4096;
    const ATTEMPTS: usize = 16;
    const BYTE_PATTERN_LENGTH: usize = 251;

    for _ in 0..ATTEMPTS {
        let output = Arc::new(TerminalSessionOutput::new(STREAM_CHUNKS));
        let writer_output = output.clone();
        let writer = std::thread::spawn(move || {
            for index in 0..STREAM_CHUNKS {
                writer_output.append_and_broadcast(&[(index % BYTE_PATTERN_LENGTH) as u8]);
            }
        });

        let received = Arc::new(Mutex::new(Vec::new()));
        let sink_received = received.clone();
        output.attach(move |bytes| {
            sink_received.lock().push(bytes.to_vec());
            true
        });
        writer.join().expect("출력 스레드 종료");

        let full: Vec<u8> = (0..STREAM_CHUNKS).map(|index| (index % BYTE_PATTERN_LENGTH) as u8).collect();
        assert_eq!(received.lock().concat(), [TERMINAL_REPLAY_PREAMBLE, full.as_slice()].concat());
    }
}
