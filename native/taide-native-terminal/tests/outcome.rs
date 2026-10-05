use taide_infra::{
    shell_integration::CommandMarker,
    terminal_scan::{OutputScanner, ScanEvent, TEXT_OVERLAP_BYTES},
};
use taide_native_terminal::{Effect, Limits, Outcome, Size, TerminalCore};

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const HISTORY: usize = 16;
const UNICODE_REPEAT: usize = 8;

fn core() -> TerminalCore {
    TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap()
}

fn stream(outcome: Outcome) -> (Vec<ScanEvent>, String, String) {
    (
        outcome
            .effects
            .into_iter()
            .map(|effect| match effect {
                Effect::Stream(event) => event,
                Effect::Terminal(_) => {
                    panic!("synthetic fixture should only produce stream effects")
                }
            })
            .collect(),
        outcome.text,
        outcome.overlap,
    )
}

#[test]
fn 실제_core의_정규화는_grid와_같은_parser에서_순서_문자경계_상한과_폐기를_보존한다() {
    let fixture = b"\x1b[3;1HDo\x1b[3;6Hyou\x1b[3;10Htrust\x1b[4;1Hthis\x1b[10Gdirectory?\r\n\x1b[1BBash\x1b[10Gcommand\x1b[1Aagain\x1b[?1049h\x1b[9;1Halternate\x1b[?47l\x1b[1;1Hprimary\x1b]2;not-text\x07\x1bPopaque\x1b\\\x1bXhidden\x1b\\\x1b^hidden\x1b\\\x1b_hidden\x1b\\\x1b(B\x1b[31mready\x1b[0m\tend\x00\x7f";
    for split in 0..=fixture.len() {
        let mut legacy = OutputScanner::new();
        let mut native = core();
        for chunk in [&fixture[..split], &fixture[split..]] {
            let outcome = legacy.scan(chunk);
            assert_eq!(
                stream(native.advance_outcome(chunk).unwrap()),
                (outcome.events, outcome.text, outcome.overlap),
                "split={split}"
            );
        }
    }
    let unicode = "한글 日本語 中文 e\u{301}𝄞".repeat(UNICODE_REPEAT);
    let mut native = core();
    let mut recovered = String::new();
    for byte in unicode.bytes() {
        let outcome = native.advance_outcome(&[byte]).unwrap();
        assert!(outcome.overlap.len() <= TEXT_OVERLAP_BYTES);
        assert!(!outcome.text.contains('\u{fffd}'));
        recovered.push_str(&outcome.text);
    }
    assert_eq!(recovered, unicode);
    assert!(unicode.ends_with(&native.advance_outcome(b"").unwrap().overlap));
    let mut sync = core();
    let pending = sync
        .advance_outcome(b"\x1b[?2026hhello\x1b]133;C\x07")
        .unwrap();
    assert!(pending.effects.is_empty());
    assert!(pending.text.is_empty());
    assert_eq!(
        stream(sync.flush_sync_outcome().unwrap()),
        (
            vec![ScanEvent::CommandMarker(CommandMarker::OutputStart)],
            "hello".into(),
            String::new()
        )
    );
    let mut overflow = core();
    overflow.advance_outcome(b"\x1b[?2026h").unwrap();
    overflow
        .advance_outcome(&vec![b'x'; Limits::default().feed_bytes])
        .unwrap();
    overflow.advance_outcome(b"y").unwrap();
    assert!(overflow.flush_sync_outcome().is_err());
    assert!(overflow.grid().is_err());
    assert!(overflow.advance_outcome(b"must-not-resume").is_err());
}
