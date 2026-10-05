use taide_infra::shell_integration::CommandMarker;
use taide_infra::terminal_scan::{OutputScanner, ScanEvent, TEXT_OVERLAP_BYTES};
use taide_terminal_core_spike::osc_effects::{
    EffectOverflow, MAX_PENDING_TEXT_BYTES, OscEffectProbe,
};

#[test]
fn tui_커서_이동과_비표시_escape는_같은_단일_parser에서_정규화된다() {
    let fixture = b"\x1b[3;1HDo\x1b[3;6Hyou\x1b[3;10Htrust\x1b[4;1Hthis\x1b[10Gdirectory?\r\n\x1b[1BBash\x1b[10Gcommand\x1b[1Aagain\x1b[?1049h\x1b[9;1Halternate\x1b[?47l\x1b[1;1Hprimary\x1b]2;not-text\x07\x1bPopaque\x1b\\\x1bXhidden\x1b\\\x1b^hidden\x1b\\\x1b_hidden\x1b\\\x1b(B\x1b[31mready\x1b[0m\tend\x00\x7f";
    for split in 0..=fixture.len() {
        let mut legacy = OutputScanner::new();
        let mut native = OscEffectProbe::default();
        for chunk in [&fixture[..split], &fixture[split..]] {
            assert_eq!(
                native.advance_outcome(chunk).unwrap(),
                legacy.scan(chunk),
                "split={split}"
            );
        }
    }
}

#[test]
fn unicode는_byte_조각에서도_유실되지_않고_overlap은_문자_경계와_상한을_지킨다() {
    const REPEAT_COUNT: usize = 32;
    let fixture = "한글 日本語 中文 e\u{301}𝄞".repeat(REPEAT_COUNT);
    let mut native = OscEffectProbe::default();
    let mut recovered = String::new();
    for byte in fixture.as_bytes() {
        let outcome = native.advance_outcome(std::slice::from_ref(byte)).unwrap();
        assert!(outcome.overlap.len() <= TEXT_OVERLAP_BYTES);
        assert!(!outcome.text.contains('\u{fffd}'));
        recovered.push_str(&outcome.text);
    }
    assert_eq!(recovered, fixture);
    let final_overlap = native.advance_outcome(b"").unwrap().overlap;
    assert!(fixture.ends_with(&final_overlap));
    let mut legacy = OutputScanner::new();
    let mut legacy_text = String::new();
    for byte in fixture.as_bytes() {
        legacy_text.push_str(&legacy.scan(std::slice::from_ref(byte)).text);
    }
    assert_ne!(legacy_text, fixture);
    assert!(legacy_text.contains('\u{fffd}'));
}

#[test]
fn 동기화된_text와_osc는_함께_보고되고_text_상한_초과는_오류다() {
    let mut terminal = OscEffectProbe::default();
    let pending = terminal
        .advance_outcome(b"\x1b[?2026hhello\x1b]133;C\x07")
        .unwrap();
    assert!(pending.text.is_empty());
    assert!(pending.events.is_empty());
    let completed = terminal.flush_sync_outcome().unwrap();
    assert_eq!(completed.text, "hello");
    assert_eq!(
        completed.events,
        [ScanEvent::CommandMarker(CommandMarker::OutputStart)]
    );
    let oversized = vec![b'x'; MAX_PENDING_TEXT_BYTES + 1];
    assert_eq!(terminal.advance_outcome(&oversized), Err(EffectOverflow));
    assert_eq!(
        terminal.advance_outcome(b"not-silently-resumed"),
        Err(EffectOverflow)
    );
}
