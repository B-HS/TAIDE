use taide_infra::shell_integration::CommandMarker;
use taide_infra::terminal_scan::{
    MAX_AGENT_EVENT_BYTES, MAX_OSC_PAYLOAD_BYTES, MAX_TITLE_BYTES, OutputScanner, ScanEvent,
};
use taide_terminal_core_spike::osc_effects::{EffectOverflow, MAX_PENDING_EVENTS, OscEffectProbe};

#[test]
fn 단일_parser의_osc_effect는_기존_scanner와_모든_분할_경계에서_일치한다() {
    const EXTRA_FIELDS: usize = 32;
    let agent_body = format!("한글;{}끝", "x;".repeat(EXTRA_FIELDS));
    let fixture = format!(
        "\x1b]7;/synthetic;한\r글\x07\x1b]7;file://host/ignored\x1b\\\x1b]133;A\x07\x1b]133;C\x1b\\\x1b]133;D; 3 \x07\x1b]133;D;invalid\x1b\\\x1b]0;첫\r제목\x07\x1b]2;title;tail\x1b\\\x1b]777;notify;taide-agent;{agent_body}\x07\x1b]777;notify;other;ignored\x1b\\\x1b]9;build;finished\x07\x1b]9;4;1;50\x1b\\\x1b]52;c;?\x07safe"
    );
    let expected = vec![
        ScanEvent::Cwd("/synthetic;한글".into()),
        ScanEvent::CommandMarker(CommandMarker::OutputStart),
        ScanEvent::CommandMarker(CommandMarker::Finished { exit_code: Some(3) }),
        ScanEvent::CommandMarker(CommandMarker::Finished { exit_code: None }),
        ScanEvent::Title("첫제목".into()),
        ScanEvent::Title("title;tail".into()),
        ScanEvent::AgentEvent(agent_body),
        ScanEvent::Notification9("build;finished".into()),
    ];
    for split in 0..=fixture.len() {
        let mut scanner = OutputScanner::new();
        let mut terminal = OscEffectProbe::default();
        let mut legacy = Vec::new();
        let mut native = Vec::new();
        for chunk in [&fixture.as_bytes()[..split], &fixture.as_bytes()[split..]] {
            legacy.extend(scanner.scan(chunk).events);
            native.extend(terminal.advance(chunk).unwrap());
        }
        assert_eq!(legacy, expected, "legacy split={split}");
        assert_eq!(native, expected, "native split={split}");
        assert_eq!(terminal.snapshot().effects.clipboard_requests, 0);
        assert!(terminal.snapshot().lines[0].contains("safe"));
    }
}

#[test]
fn payload_정책은_할당_전_크기를_거부하고_초과_osc_뒤_복구한다() {
    let fixture = format!(
        "\x1b]2;{}\x07\x1b]2;{}\r\x07\x1b]777;notify;taide-agent;{}\x1b\\\x1b]777;notify;taide-agent;{}\x07",
        "t".repeat(MAX_TITLE_BYTES),
        "t".repeat(MAX_TITLE_BYTES),
        "a".repeat(MAX_AGENT_EVENT_BYTES),
        "a".repeat(MAX_AGENT_EVENT_BYTES + 1),
    );
    let mut scanner = OutputScanner::new();
    let mut terminal = OscEffectProbe::default();
    let expected = vec![
        ScanEvent::Title("t".repeat(MAX_TITLE_BYTES)),
        ScanEvent::AgentEvent("a".repeat(MAX_AGENT_EVENT_BYTES)),
    ];
    assert_eq!(scanner.scan(fixture.as_bytes()).events, expected);
    assert_eq!(terminal.advance(fixture.as_bytes()).unwrap(), expected);
    assert!(terminal.advance(b"\x1b]7;/bad\xff\x07").unwrap().is_empty());
    assert!(terminal.advance(b"\x1b]7;").unwrap().is_empty());
    assert!(
        terminal
            .advance(&vec![b';'; MAX_OSC_PAYLOAD_BYTES])
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        terminal.advance(b"\x07\x1b]7;/recovered\x07").unwrap(),
        [ScanEvent::Cwd("/recovered".into())]
    );
}

#[test]
fn 동기화와_포화는_순서를_유지하며_효과_유실을_성공으로_보고하지_않는다() {
    let mut terminal = OscEffectProbe::default();
    assert!(
        terminal
            .advance(b"\x1b[?2026h\x1b]133;C\x07\x1b]7;/synced\x1b\\")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        terminal.flush_sync().unwrap(),
        [
            ScanEvent::CommandMarker(CommandMarker::OutputStart),
            ScanEvent::Cwd("/synced".into())
        ]
    );
    let burst = "\x1b]133;C\x07".repeat(MAX_PENDING_EVENTS);
    assert_eq!(
        terminal.advance(burst.as_bytes()).unwrap().len(),
        MAX_PENDING_EVENTS
    );
    let titles = "\x1b]2;bounded\x07".repeat(MAX_PENDING_EVENTS);
    assert_eq!(
        terminal.advance(titles.as_bytes()).unwrap().len(),
        MAX_PENDING_EVENTS
    );
    assert!(terminal.snapshot().effects.titles.is_empty());
    let oversized = "\x1b]133;C\x07".repeat(MAX_PENDING_EVENTS + 1);
    assert_eq!(terminal.advance(oversized.as_bytes()), Err(EffectOverflow));
    assert_eq!(
        terminal.advance(b"\x1b]7;/not-silently-resumed\x07"),
        Err(EffectOverflow)
    );
}
