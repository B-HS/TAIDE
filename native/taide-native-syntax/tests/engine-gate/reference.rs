use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use taide_native_syntax::{TextmateTokenizer, ThemeSetting, TokenizerLimits, bundled_grammar_set};

pub const THEME_IDS: [&str; 2] = ["one-dark-pro", "github-light"];
pub const MAX_LINE_UTF16_LENGTH: usize = 20_000;
const FIXTURES_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
const LINE_SEPARATOR: char = '\n';

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleManifest {
    pub long_line_fill: String,
    pub samples: Vec<Sample>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub id: String,
    pub language_id: String,
    pub file: String,
    pub long_line: LongLine,
}

#[derive(Deserialize)]
pub struct LongLine {
    pub prefix: String,
    pub suffix: String,
    pub tail: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeReference {
    pub settings: Vec<ThemeSetting>,
    pub color_map: Vec<Option<String>>,
    pub default_final_metadata: u32,
    pub style_scopes: Vec<StyleScope>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleScope {
    pub color_index: u32,
    pub font_style: u32,
    pub scope: String,
    pub final_metadata: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleReference {
    pub requested_language_ids: Vec<String>,
    pub loaded_scope_names: Vec<String>,
    pub line_utf16_lengths: Vec<usize>,
    pub raw_tokens: Vec<Vec<u32>>,
    pub final_tokens: Vec<Vec<u32>>,
}

fn read_fixture<T: DeserializeOwned>(relative_path: &str) -> T {
    let path = PathBuf::from(FIXTURES_DIRECTORY).join(relative_path);
    let content = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    serde_json::from_str(&content).unwrap_or_else(|error| panic!("{path:?}: {error}"))
}

pub fn sample_manifest() -> SampleManifest {
    read_fixture("samples/manifest.json")
}

pub fn theme_reference(theme_id: &str) -> ThemeReference {
    read_fixture(&format!("reference/{theme_id}/theme.json"))
}

pub fn sample_reference(theme_id: &str, sample_id: &str) -> SampleReference {
    read_fixture(&format!("reference/{theme_id}/{sample_id}.json"))
}

pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub fn authored_lines(sample: &Sample) -> Vec<String> {
    let path = PathBuf::from(FIXTURES_DIRECTORY)
        .join("samples")
        .join(&sample.file);
    let content = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    content
        .strip_suffix(LINE_SEPARATOR)
        .unwrap_or(&content)
        .split(LINE_SEPARATOR)
        .map(str::to_owned)
        .collect()
}

pub fn long_line(manifest: &SampleManifest, sample: &Sample, utf16_length: usize) -> String {
    let fill_count =
        utf16_length - utf16_len(&sample.long_line.prefix) - utf16_len(&sample.long_line.suffix);
    format!(
        "{}{}{}",
        sample.long_line.prefix,
        manifest.long_line_fill.repeat(fill_count),
        sample.long_line.suffix
    )
}

pub fn document_lines(manifest: &SampleManifest, sample: &Sample) -> Vec<String> {
    let mut lines = authored_lines(sample);
    lines.push(long_line(manifest, sample, MAX_LINE_UTF16_LENGTH - 1));
    lines.push(long_line(manifest, sample, MAX_LINE_UTF16_LENGTH));
    lines.push(sample.long_line.tail.clone());
    lines
}

pub fn tokenizer(
    theme: &ThemeReference,
    requested_language_ids: &[&str],
    limits: TokenizerLimits,
) -> TextmateTokenizer {
    let grammars = bundled_grammar_set(requested_language_ids).unwrap();
    TextmateTokenizer::new(&grammars, &theme.settings, limits).unwrap()
}

pub fn unlimited_time() -> TokenizerLimits {
    TokenizerLimits {
        line_time_limit_millis: None,
        ..TokenizerLimits::default()
    }
}
