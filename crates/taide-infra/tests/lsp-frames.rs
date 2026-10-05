use taide_infra::lsp_frame::{BoundedMessageBuffer, FrameLimits, TransportFailure};
use taide_infra::lsp_proc::encode_message;

const HEADER_LIMIT: usize = 128;
const BODY_LIMIT: usize = 32;

fn buffer() -> BoundedMessageBuffer {
    BoundedMessageBuffer::new(FrameLimits::new(HEADER_LIMIT, BODY_LIMIT).unwrap()).unwrap()
}

#[test]
fn 제한형_framing은_모든_byte_분할과_연속_utf8_프레임을_보존한다() {
    let expected = ["한글 e\u{301} 𐐀", "second"];
    let first = expected[0];
    let mut frames = format!(
        "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: {}\r\n\r\n{first}",
        first.len()
    )
    .into_bytes();
    frames.extend(encode_message(expected[1]));
    for split in 0..=frames.len() {
        let mut input = buffer();
        let mut received = Vec::new();
        for chunk in [&frames[..split], &frames[split..]] {
            input
                .push(chunk, |message| {
                    received.push(message);
                    Ok(())
                })
                .unwrap();
            assert!(input.retained_bytes() <= HEADER_LIMIT + BODY_LIMIT);
            assert!(input.allocated_capacity() <= HEADER_LIMIT + BODY_LIMIT);
        }
        input.finish().unwrap();
        assert_eq!(received, expected);
    }
    let mut input = buffer();
    let mut received = Vec::new();
    for byte in &frames {
        input
            .push(std::slice::from_ref(byte), |message| {
                received.push(message);
                Ok(())
            })
            .unwrap();
    }
    input.finish().unwrap();
    assert_eq!(received, expected);
}

#[test]
fn 상한과_잘못된_헤더는_body_할당_전에_거부하며_실패는_sticky다() {
    let cases = [
        (vec![b'x'; HEADER_LIMIT + 1], TransportFailure::HeaderTooLarge),
        (b"Other: 1\r\n\r\n".to_vec(), TransportFailure::MalformedHeader),
        (
            b"Content-Length: 1\r\nContent-Length: 1\r\n\r\n".to_vec(),
            TransportFailure::MalformedHeader,
        ),
        (b"Content-Length: -1\r\n\r\n".to_vec(), TransportFailure::MalformedHeader),
        (
            b"Content-Length: 999999999999999999999999999999999999\r\n\r\n".to_vec(),
            TransportFailure::MalformedHeader,
        ),
        (
            format!("Content-Length: {}\r\n\r\n", BODY_LIMIT + 1).into_bytes(),
            TransportFailure::BodyTooLarge,
        ),
        (
            b"Content-Length: 1\r\nContent-Type: application/vscode-jsonrpc; charset=utf-16\r\n\r\n".to_vec(),
            TransportFailure::UnsupportedCharset,
        ),
        (
            b"Content-Length: 1\r\nBad: \xff\r\n\r\n".to_vec(),
            TransportFailure::MalformedHeader,
        ),
    ];
    for (frame, failure) in cases {
        let mut input = buffer();
        assert_eq!(input.push(&frame, |_| panic!("invalid frame must not dispatch")), Err(failure));
        assert_eq!(input.retained_bytes(), 0);
        assert!(input.allocated_capacity() <= HEADER_LIMIT);
        assert_eq!(
            input.push(&encode_message("ignored"), |_| panic!("sticky failure must not dispatch")),
            Err(failure)
        );
        assert_eq!(input.finish(), Err(failure));
    }
}

#[test]
fn 정확한_body_상한과_eof_utf8_및_consumer_실패를_구분한다() {
    let mut input = buffer();
    let payload = "x".repeat(BODY_LIMIT);
    let mut received = Vec::new();
    input
        .push(&encode_message(&payload), |message| {
            received.push(message);
            Ok(())
        })
        .unwrap();
    assert_eq!(received, [payload]);
    input.finish().unwrap();
    for incomplete in [b"Content-Length: 2\r\n\r\nx".as_slice(), b"Content-Length: 2".as_slice()] {
        let mut input = buffer();
        input.push(incomplete, |_| panic!("incomplete frame must not dispatch")).unwrap();
        assert_eq!(input.finish(), Err(TransportFailure::TruncatedFrame));
        assert_eq!(input.retained_bytes(), 0);
    }
    let mut input = buffer();
    assert_eq!(
        input.push(b"Content-Length: 1\r\n\r\n\xff", |_| panic!("invalid UTF-8 must not dispatch")),
        Err(TransportFailure::InvalidUtf8)
    );
    let mut input = buffer();
    assert_eq!(
        input.push(&encode_message("bounded queue"), |_| Err(TransportFailure::ConsumerUnavailable)),
        Err(TransportFailure::ConsumerUnavailable)
    );
    assert_eq!(input.retained_bytes(), 0);
    assert!(FrameLimits::new(0, BODY_LIMIT).is_err());
    assert!(FrameLimits::new(HEADER_LIMIT, 0).is_err());
    assert!(FrameLimits::new(usize::MAX, BODY_LIMIT).is_err());
}
