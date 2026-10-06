use std::time::{Duration, Instant};

use taide_native_syntax::{LineState, TextmateTokenizer, TokenizerLimits};

use crate::reference::{
    MAX_LINE_UTF16_LENGTH, Sample, SampleManifest, THEME_IDS, authored_lines, document_lines,
    sample_manifest, theme_reference, tokenizer, unlimited_time,
};

const CORE_LANGUAGE_IDS: [&str; 3] = ["json", "jsonc", "markdown"];
const FIRST_TOKENIZATION_SAMPLE_IDS: [&str; 6] =
    ["cpp", "typescript", "ruby", "erb", "markdown", "json"];
const LARGE_DOCUMENT_SAMPLE_IDS: [&str; 5] = ["cpp", "typescript", "rust", "python", "markdown"];
const LARGE_DOCUMENT_LINES: usize = 10_000;
const LONG_LINE_OFFSET_FROM_END: usize = 3;

struct Pass {
    elapsed: Duration,
    slowest_line: Duration,
    stopped_early_lines: usize,
}

fn sample<'a>(manifest: &'a SampleManifest, sample_id: &str) -> &'a Sample {
    manifest
        .samples
        .iter()
        .find(|sample| sample.id == sample_id)
        .unwrap()
}

fn requested_language_ids(sample: &Sample) -> Vec<&str> {
    CORE_LANGUAGE_IDS
        .into_iter()
        .chain([sample.language_id.as_str()])
        .collect()
}

fn tokenize_all(tokenizer: &mut TextmateTokenizer, language_id: &str, lines: &[String]) -> Pass {
    let started = Instant::now();
    let mut slowest_line = Duration::ZERO;
    let mut stopped_early_lines = 0;
    let mut previous: Option<LineState> = None;
    for line in lines {
        let line_started = Instant::now();
        let tokenized = tokenizer
            .try_tokenize_line(language_id, line, previous.as_ref())
            .unwrap();
        slowest_line = slowest_line.max(line_started.elapsed());
        stopped_early_lines += usize::from(tokenized.is_stopped_early);
        previous = Some(tokenized.end_state);
    }
    Pass {
        elapsed: started.elapsed(),
        slowest_line,
        stopped_early_lines,
    }
}

#[test]
#[ignore]
fn 큰_문법의_문법_적재와_첫_토큰화_지연을_기록한다() {
    let manifest = sample_manifest();
    let theme = theme_reference(THEME_IDS[0]);
    for sample_id in FIRST_TOKENIZATION_SAMPLE_IDS {
        let sample = sample(&manifest, sample_id);
        let lines = authored_lines(sample);
        let started = Instant::now();
        let mut tokenizer = tokenizer(
            &theme,
            &requested_language_ids(sample),
            TokenizerLimits::default(),
        );
        let loaded = started.elapsed();
        let first_line_started = Instant::now();
        let first_line = tokenizer
            .try_tokenize_line(&sample.language_id, &lines[0], None)
            .unwrap();
        let first_line_elapsed = first_line_started.elapsed();
        let cold = tokenize_all(&mut tokenizer, &sample.language_id, &lines);
        let warm = tokenize_all(&mut tokenizer, &sample.language_id, &lines);
        println!(
            "first tokenization {sample_id}: load {loaded:?} first line {first_line_elapsed:?} (stopped early {}) cold pass {:?} ({} lines, slowest {:?}, stopped early {}) warm pass {:?} (slowest {:?})",
            first_line.is_stopped_early,
            cold.elapsed,
            lines.len(),
            cold.slowest_line,
            cold.stopped_early_lines,
            warm.elapsed,
            warm.slowest_line,
        );
    }
}

#[test]
#[ignore]
fn 만_줄_문서의_전체_토큰화_시간을_기록한다() {
    let manifest = sample_manifest();
    let theme = theme_reference(THEME_IDS[0]);
    for sample_id in LARGE_DOCUMENT_SAMPLE_IDS {
        let sample = sample(&manifest, sample_id);
        let lines: Vec<String> = authored_lines(sample)
            .into_iter()
            .cycle()
            .take(LARGE_DOCUMENT_LINES)
            .collect();
        let bytes: usize = lines.iter().map(String::len).sum();
        let mut tokenizer = tokenizer(
            &theme,
            &requested_language_ids(sample),
            TokenizerLimits::default(),
        );
        let cold = tokenize_all(&mut tokenizer, &sample.language_id, &lines);
        let warm = tokenize_all(&mut tokenizer, &sample.language_id, &lines);
        println!(
            "large document {sample_id}: lines {} bytes {bytes} cold {:?} (slowest line {:?}, stopped early {}) warm {:?} (slowest line {:?}, stopped early {})",
            lines.len(),
            cold.elapsed,
            cold.slowest_line,
            cold.stopped_early_lines,
            warm.elapsed,
            warm.slowest_line,
            warm.stopped_early_lines,
        );
    }
}

#[test]
#[ignore]
fn 한도_근처_긴_줄의_토큰화_시간과_한도_동작을_기록한다() {
    let manifest = sample_manifest();
    let theme = theme_reference(THEME_IDS[0]);
    for sample in &manifest.samples {
        let lines = document_lines(&manifest, sample);
        let long_line_index = lines.len() - LONG_LINE_OFFSET_FROM_END;
        let requested = requested_language_ids(sample);
        let measure = |limits: TokenizerLimits| {
            let mut tokenizer = tokenizer(&theme, &requested, limits);
            let mut previous: Option<LineState> = None;
            for line in &lines[..long_line_index] {
                previous = Some(
                    tokenizer
                        .try_tokenize_line(&sample.language_id, line, previous.as_ref())
                        .unwrap()
                        .end_state,
                );
            }
            let started = Instant::now();
            let below_limit = tokenizer
                .try_tokenize_line(
                    &sample.language_id,
                    &lines[long_line_index],
                    previous.as_ref(),
                )
                .unwrap();
            let below_limit_elapsed = started.elapsed();
            let started = Instant::now();
            let at_limit = tokenizer
                .try_tokenize_line(
                    &sample.language_id,
                    &lines[long_line_index + 1],
                    Some(&below_limit.end_state),
                )
                .unwrap();
            (
                below_limit_elapsed,
                below_limit.is_stopped_early,
                started.elapsed(),
                at_limit.spans.len(),
            )
        };
        let (unlimited_elapsed, _, _, _) = measure(unlimited_time());
        let (limited_elapsed, is_stopped_early, skipped_elapsed, skipped_span_fields) =
            measure(TokenizerLimits::default());
        println!(
            "long line {}: {} units without limit {unlimited_elapsed:?}, with default limit {limited_elapsed:?} (stopped early {is_stopped_early}), {MAX_LINE_UTF16_LENGTH} units skipped in {skipped_elapsed:?} ({skipped_span_fields} span fields)",
            sample.id,
            MAX_LINE_UTF16_LENGTH - 1,
        );
    }
}
