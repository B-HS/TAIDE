use std::hint::black_box;
use std::time::Instant;

use serde_json::{Value, json};
use taide_editor_core_spike::{DocumentProbe, LARGE_DOCUMENT_LINES};
use taide_lsp::native::protocol::lsp_types::{Range, TextEdit};

const COORDINATE_SAMPLES: usize = 128;
const LONG_LINE_REPEATS: usize = 4_000_000;
const PATTERN: &str = "한𐐀e\u{301}";
const INSERTION: &str = "native ";
const P95_PERCENT: usize = 95;
const PERCENT_SCALE: usize = 100;
const MEDIAN_DIVISOR: usize = 2;

fn measure(name: &str, text: String) -> Value {
    let started = Instant::now();
    let mut document = DocumentProbe::new(&text);
    let construction_ns = started.elapsed().as_nanos();
    drop(text);
    let bytes = document.rope.len_bytes();
    let chars = document.rope.len_chars();
    let units = document.rope.len_utf16_cu();
    let lines = document.rope.len_lines();
    let original = document.rope.clone();
    let row_chars = PATTERN.chars().count() + if lines > 1 { "\r\n".chars().count() } else { 0 };
    let repeats = chars / row_chars;
    let mut samples = Vec::with_capacity(COORDINATE_SAMPLES);
    for sample in 0..COORDINATE_SAMPLES {
        let scalar = sample * repeats / COORDINATE_SAMPLES * row_chars + 1;
        let byte = document.rope.char_to_byte(scalar);
        let started = Instant::now();
        let position = document.byte_to_position(0, black_box(byte)).unwrap();
        let restored = document.position_to_byte(0, black_box(position)).unwrap();
        let elapsed = started.elapsed().as_nanos();
        assert_eq!(restored, byte);
        samples.push(elapsed);
    }
    samples.sort_unstable();
    let middle = repeats / MEDIAN_DIVISOR * row_chars + 1;
    let byte = document.rope.char_to_byte(middle);
    let position = document.byte_to_position(0, byte).unwrap();
    let edit = TextEdit {
        range: Range::new(position, position),
        new_text: INSERTION.into(),
    };
    let started = Instant::now();
    document.apply_lsp_edits(0, vec![edit]).unwrap();
    let edit_ns = started.elapsed().as_nanos();
    assert_eq!(document.revision, 1);
    assert_eq!(document.rope.len_bytes(), bytes + INSERTION.len());
    assert_eq!(document.rope.len_chars(), chars + INSERTION.chars().count());
    assert_eq!(
        document.rope.len_utf16_cu(),
        units + INSERTION.encode_utf16().count()
    );
    assert_eq!(
        document
            .rope
            .slice(middle..middle + INSERTION.chars().count()),
        INSERTION
    );
    assert_eq!(original.len_bytes(), bytes);
    let edited = document.rope.clone();
    let started = Instant::now();
    assert!(document.undo().unwrap());
    let undo_ns = started.elapsed().as_nanos();
    assert_eq!(document.rope, original);
    let started = Instant::now();
    assert!(document.redo().unwrap());
    let redo_ns = started.elapsed().as_nanos();
    assert_eq!(document.rope, edited);
    let p95_index = (COORDINATE_SAMPLES * P95_PERCENT).div_ceil(PERCENT_SCALE) - 1;
    json!({
        "fixture": name,
        "bytes": bytes,
        "chars": chars,
        "utf16_units": units,
        "lines": lines,
        "construction_ns": construction_ns,
        "coordinate_roundtrip": {
            "samples": COORDINATE_SAMPLES,
            "p50_ns": samples[COORDINATE_SAMPLES / MEDIAN_DIVISOR - 1],
            "p95_ns": samples[p95_index],
            "max_ns": samples.last().unwrap(),
        },
        "edit_ns": edit_ns,
        "undo_ns": undo_ns,
        "redo_ns": redo_ns,
        "final_revision": document.revision,
        "retained_snapshots": "original and edited, verified exactly",
    })
}

fn main() {
    assert_eq!(
        std::env::args_os().count(),
        1,
        "synthetic fixture accepts no arguments"
    );
    let multiline = measure(
        "50000-crlf-lines",
        format!("{PATTERN}\r\n").repeat(LARGE_DOCUMENT_LINES),
    );
    let long_line = measure(
        "40000000-byte-single-line",
        PATTERN.repeat(LONG_LINE_REPEATS),
    );
    println!(
        "{}",
        json!({"profile": if cfg!(debug_assertions) { "debug" } else { "release" }, "fixtures": [multiline, long_line]})
    );
}
