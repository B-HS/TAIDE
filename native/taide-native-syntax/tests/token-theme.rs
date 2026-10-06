use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use taide_model::theme::{ResolvedTheme, SyntaxStyle, Theme};
use taide_native_editor::line_tokens::TokenStyle;
use taide_native_editor::syntax::TokenKind;
use taide_native_syntax::{
    FONT_STYLE_BOLD, FONT_STYLE_ITALIC, FONT_STYLE_STRIKETHROUGH, FONT_STYLE_UNDERLINE,
    FONT_STYLE_VARIANTS, GrammarSet, TextmateTokenizer, ThemeSetting, TokenTheme, TokenizerLimits,
};

const BUNDLED_THEMES_DIRECTORY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/taide-theme/resources/themes"
);
const SYNTHETIC_THEMES_DIRECTORY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/token-themes/synthetic"
);
const BUNDLED_REFERENCE_DIRECTORY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/token-themes/reference/bundled"
);
const SYNTHETIC_REFERENCE_DIRECTORY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/token-themes/reference/synthetic"
);
const THEME_FILE_EXTENSION: &str = "json";
const PALETTE_REFERENCE_PREFIX: char = '$';
const COLOR_PREFIX: char = '#';
const HEX_RADIX: u32 = 16;
const CHANNEL_DIGITS: usize = 2;
const OPAQUE_ALPHA: u8 = u8::MAX;
const METADATA_TOKEN_TYPE_OFFSET: u32 = 8;
const METADATA_TOKEN_TYPE_MASK: u32 = 0b11;
const METADATA_FONT_STYLE_OFFSET: u32 = 11;
const METADATA_FONT_STYLE_MASK: u32 = 0b1111;
const METADATA_FOREGROUND_OFFSET: u32 = 15;
const METADATA_FOREGROUND_MASK: u32 = 0b1_1111_1111;
const METADATA_COMMENT: u32 = 1;
const METADATA_STRING: u32 = 2;
const METADATA_REGEX: u32 = 3;
const BUNDLED_THEME_WITH_ILLEGAL_COLOR: &str = "intellij-islands-light";
const REJECTED_SYNTHETIC_THEMES: [&str; 5] = [
    "invalid-editor-color",
    "invalid-rule-background",
    "invalid-rule-color",
    "invalid-semantic-color",
    "invalid-syntax-color",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppliedReference {
    settings: Vec<ThemeSetting>,
    color_map: Vec<Option<String>>,
    monaco_color_map: Vec<Option<String>>,
    default_final_metadata: u32,
    final_metadata_by_style: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Reference {
    Rejected { error: String },
    Applied(AppliedReference),
}

fn read_json<T: DeserializeOwned>(path: &Path) -> T {
    let content = fs::read_to_string(path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    serde_json::from_str(&content).unwrap_or_else(|error| panic!("{path:?}: {error}"))
}

fn theme_ids(directory: &str) -> BTreeSet<String> {
    fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{directory}: {error}"))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == THEME_FILE_EXTENSION)
        })
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_owned())
        .collect()
}

fn theme_path(directory: &str, theme_id: &str) -> PathBuf {
    PathBuf::from(directory).join(format!("{theme_id}.{THEME_FILE_EXTENSION}"))
}

fn resolved_without_base(theme: Theme) -> ResolvedTheme {
    let value = |value: &String| {
        value
            .strip_prefix(PALETTE_REFERENCE_PREFIX)
            .and_then(|key| theme.palette.get(key))
            .unwrap_or(value)
            .clone()
    };
    ResolvedTheme {
        id: theme.id.clone(),
        name: theme.name.clone(),
        theme_type: theme.theme_type,
        colors: theme
            .colors
            .iter()
            .map(|(key, color)| (key.clone(), value(color)))
            .collect(),
        syntax: theme
            .syntax
            .iter()
            .map(|(key, style)| {
                (
                    key.clone(),
                    SyntaxStyle {
                        fg: value(&style.fg),
                        bold: style.bold,
                        italic: style.italic,
                    },
                )
            })
            .collect(),
        terminal: theme
            .terminal
            .iter()
            .map(|(key, color)| (key.clone(), value(color)))
            .collect(),
        token_colors: theme.token_colors.clone(),
        syntax_overrides: Vec::new(),
        warnings: Vec::new(),
        author: theme.author.clone(),
        license: theme.license.clone(),
        source: theme.source.clone(),
    }
}

fn channel(color: &str, index: usize) -> u8 {
    let start = COLOR_PREFIX.len_utf8() + index * CHANNEL_DIGITS;
    u8::from_str_radix(&color[start..start + CHANNEL_DIGITS], HEX_RADIX).unwrap()
}

fn expected_style(metadata: u32, monaco_color_map: &[Option<String>]) -> TokenStyle {
    let color_id = (metadata >> METADATA_FOREGROUND_OFFSET) & METADATA_FOREGROUND_MASK;
    let color = monaco_color_map[color_id as usize].as_deref().unwrap();
    let font_style = (metadata >> METADATA_FONT_STYLE_OFFSET) & METADATA_FONT_STYLE_MASK;
    TokenStyle {
        foreground: [
            channel(color, 0),
            channel(color, 1),
            channel(color, 2),
            OPAQUE_ALPHA,
        ],
        is_italic: font_style & FONT_STYLE_ITALIC != 0,
        is_bold: font_style & FONT_STYLE_BOLD != 0,
        is_underlined: font_style & FONT_STYLE_UNDERLINE != 0,
        is_struck_through: font_style & FONT_STYLE_STRIKETHROUGH != 0,
        kind: match (metadata >> METADATA_TOKEN_TYPE_OFFSET) & METADATA_TOKEN_TYPE_MASK {
            METADATA_COMMENT => TokenKind::Comment,
            METADATA_STRING => TokenKind::String,
            METADATA_REGEX => TokenKind::Regex,
            _ => TokenKind::Other,
        },
    }
}

fn assert_matches_reference(label: &str, resolved: &ResolvedTheme, reference: Reference) -> bool {
    let reference = match reference {
        Reference::Rejected { error } => {
            assert!(
                TokenTheme::from_resolved(resolved).is_err(),
                "{label}: ts rejects this theme with {error:?}"
            );
            return false;
        }
        Reference::Applied(reference) => reference,
    };
    let theme = TokenTheme::from_resolved(resolved).unwrap_or_else(|error| {
        panic!("{label}: {error:?}");
    });
    assert_eq!(theme.settings(), reference.settings, "{label} settings");
    let tokenizer = TextmateTokenizer::new(
        &GrammarSet::default(),
        theme.settings(),
        TokenizerLimits::default(),
    )
    .unwrap_or_else(|error| panic!("{label}: {error:?}"));
    let color_map: Vec<Option<String>> = tokenizer
        .color_map()
        .iter()
        .map(|color| (!color.is_empty()).then(|| color.clone()))
        .collect();
    assert_eq!(color_map, reference.color_map, "{label} color map");
    let table = theme
        .style_table(&tokenizer)
        .unwrap_or_else(|error| panic!("{label}: {error:?}"));
    assert_eq!(
        reference.final_metadata_by_style.len(),
        reference.color_map.len() * FONT_STYLE_VARIANTS as usize,
        "{label} reference style count"
    );
    assert_eq!(
        table.len(),
        reference.final_metadata_by_style.len(),
        "{label} style count"
    );
    assert_eq!(
        table.default_style(),
        expected_style(
            reference.default_final_metadata,
            &reference.monaco_color_map
        ),
        "{label} default style"
    );
    for (style_id, metadata) in reference.final_metadata_by_style.iter().enumerate() {
        assert_eq!(
            table.style(u32::try_from(style_id).unwrap()),
            expected_style(*metadata, &reference.monaco_color_map),
            "{label} style {style_id}"
        );
    }
    true
}

#[test]
fn 번들_테마_전체의_엔진_설정과_색_표와_스타일_표가_ts_기준과_같다() {
    let theme_ids = theme_ids(BUNDLED_THEMES_DIRECTORY);
    assert_eq!(theme_ids, self::theme_ids(BUNDLED_REFERENCE_DIRECTORY));
    let mut rejected = BTreeSet::new();
    for theme_id in &theme_ids {
        let theme: Theme = read_json(&theme_path(BUNDLED_THEMES_DIRECTORY, theme_id));
        assert!(theme.extends.is_none(), "{theme_id}");
        let reference = read_json(&theme_path(BUNDLED_REFERENCE_DIRECTORY, theme_id));
        if !assert_matches_reference(theme_id, &resolved_without_base(theme), reference) {
            rejected.insert(theme_id.as_str());
        }
    }
    assert_eq!(rejected, BTreeSet::from([BUNDLED_THEME_WITH_ILLEGAL_COLOR]));
}

#[test]
fn 합성_테마의_ts_고유_동작과_거절_조건이_ts_기준과_같다() {
    let theme_ids = theme_ids(SYNTHETIC_THEMES_DIRECTORY);
    assert_eq!(theme_ids, self::theme_ids(SYNTHETIC_REFERENCE_DIRECTORY));
    let mut rejected = BTreeSet::new();
    for theme_id in &theme_ids {
        let resolved: ResolvedTheme = read_json(&theme_path(SYNTHETIC_THEMES_DIRECTORY, theme_id));
        let reference = read_json(&theme_path(SYNTHETIC_REFERENCE_DIRECTORY, theme_id));
        if !assert_matches_reference(theme_id, &resolved, reference) {
            rejected.insert(theme_id.as_str());
        }
    }
    assert_eq!(rejected, BTreeSet::from(REJECTED_SYNTHETIC_THEMES));
}
