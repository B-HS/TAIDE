use std::sync::{Arc, Mutex};

use alacritty_terminal::vte::ansi::{Handler, OscObserver, Processor, StdSyncHandler};
use alacritty_terminal::vte::{Parser, Perform};

const OSC_LIMIT: usize = 64;
const OVERSIZED_CHUNKS: usize = 1_000;

#[derive(Debug, Default)]
struct CapturedOsc {
    params: Vec<Vec<Vec<u8>>>,
}

impl Perform for CapturedOsc {
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        self.params
            .push(params.iter().map(|part| part.to_vec()).collect());
    }
}

#[derive(Debug)]
struct Observer(Arc<Mutex<Vec<Vec<Vec<u8>>>>>);

impl OscObserver for Observer {
    fn observe(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        self.0
            .lock()
            .unwrap()
            .push(params.iter().map(|part| part.to_vec()).collect());
    }
}

#[derive(Default)]
struct NoopHandler;
impl Handler for NoopHandler {}

#[derive(Debug)]
struct PayloadObserver(Arc<Mutex<Vec<Vec<u8>>>>);

impl OscObserver for PayloadObserver {
    fn observe(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn observe_payload(&mut self, payload: &[u8], _bell_terminated: bool) {
        self.0.lock().unwrap().push(payload.to_vec());
    }
}

#[test]
fn 원형_osc는_구분자와_제어문자를_보존하고_완전한_종료_뒤에만_보고된다() {
    const EXTRA_FIELDS: usize = 32;
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut processor = Processor::<StdSyncHandler>::new();
    processor.set_osc_observer(Some(Box::new(PayloadObserver(events.clone()))));
    let mut handler = NoopHandler;
    let payload = format!("777;notify;taide-agent;한\r{}끝", "x;".repeat(EXTRA_FIELDS));
    let fixture = format!("\x1b]{payload}\x1b");
    for byte in fixture.as_bytes() {
        processor.advance(&mut handler, std::slice::from_ref(byte));
    }
    assert!(events.lock().unwrap().is_empty());
    processor.advance(&mut handler, b"\\");
    assert_eq!(*events.lock().unwrap(), [payload.as_bytes()]);
    processor.advance(
        &mut handler,
        b"\x1b]7;/abandoned\x18\x1b]7;/incomplete\x1b[H",
    );
    assert_eq!(events.lock().unwrap().len(), 1);
    processor.advance(&mut handler, b"\x1b]7;/complete\x07");
    assert_eq!(events.lock().unwrap().last().unwrap(), b"7;/complete");
}

#[test]
fn 구분자와_무시되는_제어문자도_osc_상한에_포함된다() {
    let mut parser = Parser::<OSC_LIMIT>::default();
    let mut captured = CapturedOsc::default();
    parser.advance(&mut captured, b"\x1b]2;");
    for byte in *b";\r" {
        parser.advance(&mut captured, &[byte; OSC_LIMIT]);
        assert!(parser.osc_buffer_len() <= OSC_LIMIT);
    }
    parser.advance(&mut captured, b"\x07\x1b]2;valid\x07");
    assert_eq!(
        captured.params,
        vec![vec![b"2".to_vec(), b"valid".to_vec()]]
    );
}

#[test]
fn 끝나지_않은_osc는_상한을_유지하고_잘린_명령을_실행하지_않는다() {
    let mut parser = Parser::<OSC_LIMIT>::default();
    let mut captured = CapturedOsc::default();
    parser.advance(&mut captured, b"\x1b]2;");
    for _ in 0..OVERSIZED_CHUNKS {
        parser.advance(&mut captured, &[b'x'; OSC_LIMIT]);
        assert!(parser.osc_buffer_len() <= OSC_LIMIT);
    }
    parser.advance(&mut captured, b"\x07\x1b]2;valid\x1b\\");
    assert_eq!(
        captured.params,
        vec![vec![b"2".to_vec(), b"valid".to_vec()]]
    );
}

#[test]
fn 제품_osc는_하나의_ansi_parser_callback에서_순서대로_수집된다() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut processor = Processor::<StdSyncHandler>::new();
    processor.set_osc_observer(Some(Box::new(Observer(events.clone()))));
    let mut handler = NoopHandler;
    let fixture = b"\x1b]7;/synthetic\x07\x1b]133;C\x1b\\\x1b]777;notify;taide-agent;synthetic\x07";
    for byte in fixture {
        processor.advance(&mut handler, std::slice::from_ref(byte));
    }
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], [b"7".to_vec(), b"/synthetic".to_vec()]);
    assert_eq!(events[1], [b"133".to_vec(), b"C".to_vec()]);
    assert_eq!(
        events[2],
        [
            b"777".to_vec(),
            b"notify".to_vec(),
            b"taide-agent".to_vec(),
            b"synthetic".to_vec()
        ]
    );
}

#[test]
fn 동기화된_update도_같은_observer를_사용하고_제거_뒤에는_호출하지_않는다() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut processor = Processor::<StdSyncHandler>::new();
    processor.set_osc_observer(Some(Box::new(Observer(events.clone()))));
    let mut handler = NoopHandler;
    processor.advance(&mut handler, b"\x1b[?2026h\x1b]7;/synthetic\x07");
    assert!(events.lock().unwrap().is_empty());
    processor.stop_sync(&mut handler);
    assert_eq!(events.lock().unwrap().len(), 1);
    processor.set_osc_observer(None);
    processor.advance(&mut handler, b"\x1b]7;/another\x07");
    assert_eq!(events.lock().unwrap().len(), 1);
}
