use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::encode_message;
use taide_infra::lsp_writer::{QueuedWriter, WriterFailure, WriterLimits};
use tokio::io::{duplex, AsyncReadExt};
use tokio::time::timeout;

const HEADER_LIMIT: usize = 128;
const BODY_LIMIT: usize = 1024;
const BYTE_LIMIT: usize = 4096;
const PIPE_CAPACITY: usize = 8;
const QUEUE_CAPACITY: usize = 2;
const TEST_TIMEOUT: Duration = Duration::from_secs(3);

#[tokio::test]
async fn write_ack_취소는_진행중인_프레임을_잘라내지_않고_후속_프레임_순서를_유지한다() {
    let (output, mut input) = duplex(PIPE_CAPACITY);
    let (writer, mut owner) = QueuedWriter::start(
        output,
        FrameLimits::new(HEADER_LIMIT, BODY_LIMIT).unwrap(),
        WriterLimits::new(QUEUE_CAPACITY, BYTE_LIMIT).unwrap(),
        || panic!("정상 transport가 실패하면 안 됩니다"),
    )
    .unwrap();
    let first_payload = "한글 e\u{301} 𐐷".repeat(PIPE_CAPACITY);
    let first = encode_message(&first_payload);
    let second = encode_message("다음");
    let first_receipt = writer.submit(&first_payload).unwrap();
    let waiting = tokio::spawn(first_receipt.wait());
    let mut prefix = [0; PIPE_CAPACITY];
    timeout(TEST_TIMEOUT, input.read_exact(&mut prefix)).await.unwrap().unwrap();
    assert_eq!(prefix, first[..PIPE_CAPACITY]);
    assert!(!waiting.is_finished());
    waiting.abort();
    assert!(waiting.await.unwrap_err().is_cancelled());
    let second_receipt = writer.submit("다음").unwrap();
    let mut remaining = vec![0; first.len() - PIPE_CAPACITY + second.len()];
    timeout(TEST_TIMEOUT, input.read_exact(&mut remaining)).await.unwrap().unwrap();
    assert_eq!(remaining, [first[PIPE_CAPACITY..].to_vec(), second].concat());
    assert_eq!(timeout(TEST_TIMEOUT, second_receipt.wait()).await.unwrap(), Ok(()));
    assert_eq!(writer.retained_frame_bytes(), 0);
    timeout(TEST_TIMEOUT, owner.finish()).await.unwrap();
    assert_eq!(writer.submit("{} ").err(), Some(WriterFailure::Closed));
    assert_eq!(input.read(&mut prefix).await.unwrap(), 0);
}

#[tokio::test]
async fn queue와_inflight_byte_상한은_할당_전에_거절하고_종료시_모든_budget을_반환한다() {
    let payload = "한글".repeat(PIPE_CAPACITY);
    let frame_length = encode_message(&payload).len();
    for (capacity, byte_limit, expected) in [
        (1, BYTE_LIMIT, WriterFailure::QueueFull),
        (QUEUE_CAPACITY, frame_length, WriterFailure::ByteBudgetFull),
    ] {
        let (output, mut input) = duplex(1);
        let (writer, mut owner) = QueuedWriter::start(
            output,
            FrameLimits::new(HEADER_LIMIT, BODY_LIMIT).unwrap(),
            WriterLimits::new(capacity, byte_limit).unwrap(),
            || panic!("관리자가 닫은 transport는 write failure가 아닙니다"),
        )
        .unwrap();
        let first = writer.submit(&payload).unwrap();
        let mut first_byte = [0];
        timeout(TEST_TIMEOUT, input.read_exact(&mut first_byte)).await.unwrap().unwrap();
        let second = if expected == WriterFailure::QueueFull {
            Some(writer.submit(&payload).unwrap())
        } else {
            None
        };
        let retained = writer.retained_frame_bytes();
        assert_eq!(writer.submit(&payload).err(), Some(expected));
        assert_eq!(writer.retained_frame_bytes(), retained);
        assert!(retained <= byte_limit);
        assert_eq!(writer.submit(&"x".repeat(BODY_LIMIT + 1)).err(), Some(WriterFailure::FrameTooLarge));
        timeout(TEST_TIMEOUT, owner.finish()).await.unwrap();
        assert_eq!(first.wait().await, Err(WriterFailure::Closed));
        if let Some(second) = second {
            assert_eq!(second.wait().await, Err(WriterFailure::Closed));
        }
        assert_eq!(writer.retained_frame_bytes(), 0);
        assert_eq!(writer.submit("{}").err(), Some(WriterFailure::Closed));
    }
    assert_eq!(WriterLimits::new(0, BYTE_LIMIT).err(), Some(WriterFailure::InvalidLimits));
    assert_eq!(WriterLimits::new(1, usize::MAX).err(), Some(WriterFailure::InvalidLimits));
}

#[tokio::test]
async fn broken_pipe는_한번_실패를_보고하고_pending_ack와_writer를_회수한다() {
    let (output, input) = duplex(PIPE_CAPACITY);
    drop(input);
    let failures = Arc::new(AtomicUsize::new(0));
    let observed = failures.clone();
    let (writer, mut owner) = QueuedWriter::start(
        output,
        FrameLimits::new(HEADER_LIMIT, BODY_LIMIT).unwrap(),
        WriterLimits::new(QUEUE_CAPACITY, BYTE_LIMIT).unwrap(),
        move || {
            observed.fetch_add(1, Ordering::SeqCst);
        },
    )
    .unwrap();
    let first = writer.submit("{}").unwrap();
    let second = writer.submit("{}").unwrap();
    assert_eq!(timeout(TEST_TIMEOUT, first.wait()).await.unwrap(), Err(WriterFailure::WriteFailed));
    assert_eq!(timeout(TEST_TIMEOUT, second.wait()).await.unwrap(), Err(WriterFailure::Closed));
    timeout(TEST_TIMEOUT, owner.finish()).await.unwrap();
    assert_eq!(failures.load(Ordering::SeqCst), 1);
    assert_eq!(writer.retained_frame_bytes(), 0);
    assert_eq!(writer.submit("{}").err(), Some(WriterFailure::Closed));
}
