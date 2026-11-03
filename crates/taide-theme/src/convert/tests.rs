use serde_json::{json, Value};
use taide_model::theme::{ThemeType, TokenColorRule, TokenColorSettings};

use super::color::{contrast_ratio, delta_e76};
use super::contrast::{repair_component_contrast, repair_contrast_pairs, validate_output_colors};
use super::jsonc::{parse_jsonc, strip_json_comments};
use super::mapping::{is_distinct_from_body_foreground, is_opaque_foreground_candidate};
use super::state::{repair_state_distinctness, validate_state_distinctness};
use super::tables::{
    APP_SHADOW_MIN_ALPHA, COLOR_NAMESPACES, COMPONENT_CONTRAST_PAIRS, STATE_DISTINCTNESS_PAIRS, STATE_MIN_DISTINCT_DELTA_E,
    SUBTLE_STATE_MIN_DISTINCT_DELTA_E, SYNTAX_SCOPE_CANDIDATES,
};
use super::{convert_vscode_theme, Colors, ConversionResult};

const STATE_PAIRS_SOURCE: &str = include_str!("../../../../src/shared/lib/theme-convert/state-distinctness-pairs.ts");
const COMPONENT_PAIRS_SOURCE: &str = include_str!("../../../../src/shared/lib/theme-convert/component-contrast-pairs.ts");
const VOCABULARY_SOURCE: &str = include_str!("../../../../src/shared/lib/theme-convert/ui-token-vocabulary.ts");
const LABEL_MARKER: &str = "label: '";

fn minimal_dark() -> Value {
    json!({
        "colors": {
            "editor.background": "#1e1e1e",
            "editor.foreground": "#d4d4d4",
            "foreground": "#d4d4d4",
        }
    })
}

fn with(mut base: Value, key: &str, value: Value) -> Value {
    base[key] = value;
    base
}

fn colors(entries: &[(&str, &str)]) -> Value {
    json!({ "colors": entries.iter().map(|(key, value)| ((*key).to_owned(), json!(value))).collect::<serde_json::Map<_, _>>() })
}

fn dark(source: Value) -> ConversionResult {
    convert_vscode_theme(&[source], ThemeType::Dark)
}

fn ts_labels(source: &str, table: &str) -> Vec<String> {
    let start = source
        .find(&format!("export const {table}"))
        .expect("table exists in the TS source");
    let end = source[start..].find("\n]").map_or(source.len(), |end| start + end);
    source[start..end]
        .match_indices(LABEL_MARKER)
        .map(|(index, _)| {
            let rest = &source[start + index + LABEL_MARKER.len()..];
            rest[..rest.find('\'').expect("label ends")].to_owned()
        })
        .collect()
}

#[test]
fn jsonc는_주석과_끝_쉼표를_지우고_문자열_안은_보존한다() {
    assert_eq!(strip_json_comments("{ \"a\": 1 // comment\n}"), "{ \"a\": 1 \n}");
    assert_eq!(strip_json_comments("{ /* comment */ \"a\": 1 }"), "{  \"a\": 1 }");
    assert_eq!(
        strip_json_comments("{ \"a\": \"https://example.com\" }"),
        "{ \"a\": \"https://example.com\" }"
    );
    assert_eq!(strip_json_comments("{ \"a\": 1, }"), "{ \"a\": 1 }");
    assert_eq!(strip_json_comments("[1, 2, ]"), "[1, 2 ]");
    assert_eq!(strip_json_comments("{ \"a\": \",}\" }"), "{ \"a\": \",}\" }");
    let source = "{\n  // theme colors\n  \"colors\": {\n    \"editor.background\": \"#1e1e1e\", // dark bg\n  },\n}";
    assert_eq!(
        parse_jsonc(source).unwrap(),
        json!({ "colors": { "editor.background": "#1e1e1e" } })
    );
}

#[test]
fn 전경_후보_판정은_축약_알파와_본문_전경_거리를_원본대로_본다() {
    assert!(!is_opaque_foreground_candidate(Some("#fd03")));
    assert!(is_opaque_foreground_candidate(Some("#fd0f")));
    for same in ["#f8f8f2", "#403f53", "#ffffff"] {
        assert!(!is_distinct_from_body_foreground(same, same));
    }
    assert!(is_distinct_from_body_foreground("#C5C5C5", "#D4D4D4"));
    assert!(is_distinct_from_body_foreground("#ffd33d", "#d1d5da"));
}

#[test]
fn 어휘와_쌍_표는_rust_필수_키와_ts_정본과_일치한다() {
    let namespaced = COLOR_NAMESPACES
        .iter()
        .flat_map(|(namespace, tokens)| tokens.iter().map(move |token| format!("{namespace}.{token}")))
        .collect::<std::collections::BTreeSet<_>>();
    let required = crate::service::required_color_keys()
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(namespaced, required);
    let syntax = SYNTAX_SCOPE_CANDIDATES.iter().map(|(token, _)| *token).collect::<Vec<_>>();
    assert_eq!(
        syntax.iter().copied().collect::<std::collections::BTreeSet<_>>(),
        crate::service::required_syntax_keys().into_iter().collect()
    );
    for (token, _) in SYNTAX_SCOPE_CANDIDATES {
        assert!(VOCABULARY_SOURCE.contains(&format!("    {token}: [")), "{token}");
    }
    assert_eq!(
        STATE_DISTINCTNESS_PAIRS
            .iter()
            .map(|pair| pair.label.to_owned())
            .collect::<Vec<_>>(),
        ts_labels(STATE_PAIRS_SOURCE, "STATE_DISTINCTNESS_PAIRS")
    );
    assert_eq!(
        COMPONENT_CONTRAST_PAIRS
            .iter()
            .map(|pair| pair.label.to_owned())
            .collect::<Vec<_>>(),
        ts_labels(COMPONENT_PAIRS_SOURCE, "COMPONENT_CONTRAST_PAIRS")
    );
}

#[test]
fn 최소_입력도_전량을_채워_완전하고_safe_default와_ansi_대체를_기록한다() {
    let result = dark(minimal_dark());
    assert!(result.is_complete);
    assert!(result.missing_colors.is_empty());
    assert!(result.missing_syntax.is_empty());
    assert!(result.missing_terminal.is_empty());
    assert!(result.output_color_errors.is_empty());
    assert_eq!(result.colors["editor.background"], "#1e1e1e");
    assert_eq!(result.colors["editor.foreground"], "#d4d4d4");
    assert!(!result.safe_default_notices.is_empty());
    assert_eq!(result.ansi_fallback_tokens.len(), 16);
    assert_eq!(result.terminal["green"], "#0dbc79");
}

#[test]
fn include_체인과_token_colors는_base를_먼저_leaf를_나중에_병합한다() {
    let base = colors(&[
        ("editor.background", "#000000"),
        ("editor.foreground", "#d4d4d4"),
        ("foreground", "#d4d4d4"),
    ]);
    let result = convert_vscode_theme(&[base, colors(&[("editor.background", "#111111")])], ThemeType::Dark);
    assert_eq!(result.colors["editor.background"], "#111111");

    let specific = with(
        minimal_dark(),
        "tokenColors",
        json!([
            { "scope": "keyword", "settings": { "foreground": "#ff0000" } },
            { "scope": "keyword.control", "settings": { "foreground": "#00ff00" } },
        ]),
    );
    assert_eq!(dark(specific).syntax["keyword"].fg, "#00ff00");
    let repeated = with(
        minimal_dark(),
        "tokenColors",
        json!([
            { "scope": "keyword.control", "settings": { "foreground": "#ff0000" } },
            { "scope": "keyword.control", "settings": { "foreground": "#00ff00" } },
        ]),
    );
    assert_eq!(dark(repeated).syntax["keyword"].fg, "#00ff00");
    let base = with(
        minimal_dark(),
        "tokenColors",
        json!([{ "scope": "comment", "settings": { "foreground": "#6a9955" } }]),
    );
    let leaf = json!({ "tokenColors": [{ "scope": "keyword", "settings": { "foreground": "#ff0000" } }] });
    let result = convert_vscode_theme(&[base, leaf], ThemeType::Dark);
    assert_eq!(
        result.token_colors.iter().map(|rule| rule.scope.clone()).collect::<Vec<_>>(),
        vec![vec!["comment".to_owned()], vec!["keyword".to_owned()]]
    );
}

#[test]
fn token_colors는_font_style_원문과_background와_전역_룰을_보존하고_빈_룰을_뺀다() {
    let source = with(
        minimal_dark(),
        "tokenColors",
        json!([
            { "scope": "comment", "settings": { "foreground": "#6a9955", "fontStyle": "italic underline" } },
            { "scope": "markup.inserted", "settings": { "background": "#003300" } },
            { "settings": { "foreground": "#d4d4d4" } },
            { "scope": "string", "settings": {} },
        ]),
    );
    let result = dark(source);
    let rule = |scope: &[&str], foreground: Option<&str>, background: Option<&str>, style: Option<&str>| TokenColorRule {
        scope: scope.iter().map(|scope| (*scope).to_owned()).collect(),
        settings: TokenColorSettings {
            foreground: foreground.map(str::to_owned),
            background: background.map(str::to_owned),
            font_style: style.map(str::to_owned),
        },
    };
    assert_eq!(
        result.token_colors,
        vec![
            rule(&["comment"], Some("#6a9955"), None, Some("italic underline")),
            rule(&["markup.inserted"], None, Some("#003300"), None),
            rule(&[], Some("#d4d4d4"), None, None),
        ]
    );
    assert!(result.syntax["comment"].italic);
}

#[test]
fn match_highlight는_반투명과_본문_동일색을_배제하고_safe_default로_떨어진다() {
    assert_eq!(dark(minimal_dark()).colors["panel.matchHighlight"], "#569CD6");
    let translucent = colors(&[
        ("editor.background", "#1e1e1e"),
        ("editor.foreground", "#d4d4d4"),
        ("foreground", "#d4d4d4"),
        ("editor.findMatchHighlightBackground", "#ffd33d22"),
    ]);
    let result = dark(translucent);
    assert_eq!(result.colors["panel.matchHighlight"], "#569CD6");
    assert!(result.output_color_errors.is_empty());
    let light = colors(&[
        ("editor.background", "#ffffff"),
        ("editor.foreground", "#1e1e1e"),
        ("foreground", "#1e1e1e"),
        ("editor.findMatchHighlightBackground", "#ffdf5d66"),
    ]);
    assert_eq!(
        convert_vscode_theme(&[light], ThemeType::Light).colors["panel.matchHighlight"],
        "#0066BF"
    );
    let opaque = colors(&[
        ("editor.background", "#1e1e1e"),
        ("editor.foreground", "#d4d4d4"),
        ("foreground", "#d4d4d4"),
        ("list.highlightForeground", "#ffffff"),
        ("editor.findMatchHighlightBackground", "#ffd33d22"),
    ]);
    assert_eq!(dark(opaque).colors["panel.matchHighlight"], "#ffffff");
    let monokai = colors(&[
        ("editor.background", "#272822"),
        ("editor.foreground", "#f8f8f2"),
        ("foreground", "#f8f8f2"),
        ("list.highlightForeground", "#f8f8f2"),
    ]);
    let result = dark(monokai);
    assert_eq!(result.colors["panel.matchHighlight"], "#569CD6");
    assert!(result.output_color_errors.is_empty());
    let near = colors(&[
        ("editor.background", "#1e1e1e"),
        ("editor.foreground", "#D4D4D4"),
        ("foreground", "#D4D4D4"),
        ("list.highlightForeground", "#C5C5C5"),
    ]);
    assert_eq!(dark(near).colors["panel.matchHighlight"], "#C5C5C5");
}

#[test]
fn 대비_수리는_구별_후보가_없으면_본문_전경으로_떨어지며_고지를_남기고_있으면_그_색을_쓴다() {
    let nord = [
        ("foreground", "#D8DEE9"),
        ("editor.foreground", "#D8DEE9"),
        ("editor.background", "#2E3440"),
        ("sideBar.background", "#5A5A5A"),
        ("list.highlightForeground", "#D8DEE9"),
    ];
    let result = dark(colors(&nord));
    assert_eq!(result.colors["panel.matchHighlight"], "#D8DEE9");
    assert!(result.output_color_errors.is_empty());
    assert!(result
        .repairs
        .iter()
        .any(|repair| { repair.starts_with("panel.matchHighlight") && repair.contains("구별 가능한 후보 없음") }));
    let with_link = nord.into_iter().chain([("textLink.foreground", "#88C0D0")]).collect::<Vec<_>>();
    let result = dark(colors(&with_link));
    assert_eq!(result.colors["panel.matchHighlight"], "#88C0D0");
    assert!(!result.repairs.iter().any(|repair| repair.contains("구별 가능한 후보 없음")));
    let everforest = colors(&[
        ("editor.background", "#fdf6e3"),
        ("editor.foreground", "#5c6a72"),
        ("foreground", "#5c6a72"),
        ("sideBar.background", "#fdf6e3"),
        ("list.highlightForeground", "#8da101"),
    ]);
    assert!(convert_vscode_theme(&[everforest], ThemeType::Light).output_color_errors.is_empty());
    let rose_pine = colors(&[
        ("editor.background", "#faf4ed"),
        ("editor.foreground", "#575279"),
        ("foreground", "#575279"),
        ("sideBar.background", "#faf4ed"),
        ("list.highlightForeground", "#d7827e"),
        ("textLink.foreground", "#907aa9"),
    ]);
    let result = convert_vscode_theme(&[rose_pine], ThemeType::Light);
    assert_eq!(result.colors["panel.matchHighlight"], "#907aa9");
    assert!(result.output_color_errors.is_empty());
}

#[test]
fn 최소_light_입력도_전량을_채워_대비_오류_없이_변환한다() {
    let light = colors(&[
        ("editor.background", "#ffffff"),
        ("editor.foreground", "#1e1e1e"),
        ("foreground", "#1e1e1e"),
    ]);
    let result = convert_vscode_theme(&[light], ThemeType::Light);
    assert!(result.is_complete);
    assert!(result.output_color_errors.is_empty(), "{:?}", result.output_color_errors);
    assert_eq!(result.terminal["green"], "#107c10");
}

#[test]
fn 상태색_구별성은_바탕과_같은_상태색을_수리하고_terminal_미러에도_반영한다() {
    let result = dark(colors(&[
        ("editor.background", "#1e1e1e"),
        ("editor.foreground", "#d4d4d4"),
        ("foreground", "#d4d4d4"),
        ("editor.selectionBackground", "#1e1e1e"),
    ]));
    assert_ne!(result.colors["editor.selection"], "#1e1e1e");
    assert!(result.repairs.iter().any(|repair| repair.contains("editorSelection 구별성 확보")));
    assert!(result.state_distinctness_errors.is_empty());
    let result = dark(colors(&[
        ("editor.background", "#1e1e1e"),
        ("editor.foreground", "#d4d4d4"),
        ("foreground", "#d4d4d4"),
        ("terminal.background", "#1e1e1e"),
        ("terminal.selectionBackground", "#1e1e1e"),
    ]));
    assert_eq!(result.terminal["selection"], result.colors["terminal.selection"]);
    assert_ne!(result.terminal["selection"], "#1e1e1e");
}

const STATE_BASE_COLORS: &[(&str, &str)] = &[
    ("app.background", "#1e1e2e"),
    ("app.foreground", "#cdd6f4"),
    ("app.focusBorder", "#89b4fa"),
    ("app.shadow", "#00000066"),
    ("appSidebar.background", "#181825"),
    ("appSidebar.itemHover", "#313244"),
    ("appSidebar.itemActive", "#45475a"),
    ("appSidebar.badge", "#f38ba8"),
    ("tabBar.background", "#181825"),
    ("tabBar.tabActiveBackground", "#1e1e2e"),
    ("tabBar.tabInactiveBackground", "#11111b"),
    ("tabBar.tabActiveIndicator", "#89b4fa"),
    ("explorer.background", "#181825"),
    ("explorer.itemHover", "#313244"),
    ("explorer.itemSelected", "#585b70"),
    ("explorer.itemFocused", "#45475a"),
    ("panel.background", "#181825"),
    ("panel.inputBackground", "#1e1e2e"),
    ("panel.inputBorder", "#45475a"),
    ("editor.background", "#1e1e2e"),
    ("editor.widgetBackground", "#181825"),
    ("editor.lineHighlight", "#313244"),
    ("editor.cursor", "#f5e0dc"),
    ("editor.selection", "#45475a"),
    ("editor.inactiveSelection", "#313244"),
    ("editor.bracketMatch", "#f9e2af"),
    ("editor.findMatch", "#f9e2af"),
    ("editor.findMatchHighlight", "#585b70"),
    ("terminal.background", "#1e1e2e"),
    ("terminal.cursor", "#f5e0dc"),
    ("terminal.selection", "#45475a"),
    ("menu.background", "#181825"),
    ("menu.itemHover", "#45475a"),
    ("modal.background", "#181825"),
    ("modal.itemHover", "#45475a"),
    ("scrollbar.thumb", "#45475a"),
    ("scrollbar.track", "transparent"),
    ("input.background", "#1e1e2e"),
    ("input.border", "#585b70"),
    ("input.focusBorder", "#89b4fa"),
    ("button.background", "#313244"),
    ("button.hoverBackground", "#45475a"),
    ("button.primaryBackground", "#89b4fa"),
    ("list.background", "#181825"),
    ("list.hoverBackground", "#313244"),
    ("list.activeBackground", "#45475a"),
];

const CONTRAST_BASE_COLORS: &[(&str, &str)] = &[
    ("app.foreground", "#d4d4d4"),
    ("app.background", "#1e1e1e"),
    ("editor.foreground", "#d4d4d4"),
    ("editor.background", "#1e1e1e"),
    ("panel.sectionHeader", "#d4d4d4"),
    ("panel.background", "#000000"),
    ("tooltip.background", "#1e1e1e"),
    ("panel.matchHighlight", "#569cd6"),
    ("list.foreground", "#d4d4d4"),
    ("list.background", "#1e1e1e"),
    ("list.hoverBackground", "#1e1e1e"),
    ("list.activeBackground", "#1e1e1e"),
];

const ACCENT_MENU_COLORS: &[(&str, &str)] = &[
    ("app.background", "#ffffff"),
    ("app.foreground", "#3b3b3b"),
    ("menu.background", "#ffffff"),
    ("menu.itemHover", "#005fb8"),
    ("list.hoverBackground", "#f1f1f1"),
];

const AYU_SIBLING_OVERRIDES: &[(&str, &str)] = &[
    ("app.foreground", "#5a6378"),
    ("explorer.background", "#0d1017"),
    ("explorer.itemHover", "#47526640"),
    ("explorer.itemSelected", "#47526640"),
    ("explorer.itemFocused", "#47526640"),
];

const STATE_REPAIR_DELTA_E_HEADROOM: f64 = 1.0;
const COMPONENT_REPAIR_MAX_DELTA_E: f64 = 5.0;
const MIN_COMPONENT_CONTRAST: f64 = 3.0;

fn colors_with(base: &[(&str, &str)], overrides: &[(&str, &str)]) -> Colors {
    let mut colors = Colors::new();
    for (key, value) in base.iter().chain(overrides) {
        colors.insert((*key).into(), (*value).into());
    }
    colors
}

fn state(overrides: &[(&str, &str)]) -> Colors {
    colors_with(STATE_BASE_COLORS, overrides)
}

fn contrast_colors(overrides: &[(&str, &str)]) -> Colors {
    colors_with(CONTRAST_BASE_COLORS, overrides)
}

fn first_words(messages: &[String]) -> Vec<&str> {
    messages
        .iter()
        .map(|message| message.split(' ').next().unwrap_or_default())
        .collect()
}

fn repaired_state(mut colors: Colors) -> (Colors, Vec<String>) {
    let repairs = repair_state_distinctness(&mut colors);
    (colors, repairs)
}

fn repaired_contrast(mut colors: Colors, vscode: &[(&str, &str)]) -> (Colors, Vec<String>) {
    let repairs = repair_contrast_pairs(&mut colors, &colors_with(vscode, &[]));
    (colors, repairs)
}

fn repaired_components(mut colors: Colors) -> (Colors, Vec<String>) {
    let repairs = repair_component_contrast(&mut colors);
    (colors, repairs)
}

#[test]
fn 상태_구별성_검증은_ts_사례와_같은_쌍을_잡는다() {
    assert!(validate_state_distinctness(&state(&[])).is_empty());
    assert_eq!(
        first_words(&validate_state_distinctness(&state(&[("editor.selection", "#1e1e2e")]))),
        ["editorSelection"]
    );
    assert_eq!(
        first_words(&validate_state_distinctness(&state(&[("terminal.selection", "#1e1e2e80")]))),
        ["terminalSelection"]
    );
    assert!(validate_state_distinctness(&state(&[
        ("explorer.background", "#21222c"),
        ("explorer.itemHover", "#44475a75"),
        ("explorer.itemSelected", "#44475a"),
        ("explorer.itemFocused", "#44475a75"),
    ]))
    .is_empty());
    assert!(!first_words(&validate_state_distinctness(&state(&[("scrollbar.thumb", "#1e1e2e")]))).contains(&"scrollbarThumb"));
    assert_eq!(
        first_words(&validate_state_distinctness(&state(&[("editor.lineHighlight", "#1e1e2e")]))),
        ["editorCurrentLine"]
    );
    assert!(validate_state_distinctness(&state(&[("editor.lineHighlight", "#222232")])).is_empty());
    assert!(SUBTLE_STATE_MIN_DISTINCT_DELTA_E < STATE_MIN_DISTINCT_DELTA_E);
    assert!(first_words(&validate_state_distinctness(&state(AYU_SIBLING_OVERRIDES))).contains(&"explorerSelectedVsHover"));
    let everforest = [
        ("list.background", "#fdf6e3"),
        ("list.hoverBackground", "#f5efde"),
        ("list.activeBackground", "#e6e2cc80"),
        ("tabBar.tabActiveBackground", "#fdf6e3"),
        ("tabBar.tabInactiveBackground", "#fdf6e3"),
    ];
    assert!(!first_words(&validate_state_distinctness(&state(&everforest))).contains(&"listActiveVsHover"));
    let collapsed = [everforest.as_slice(), &[("list.activeBackground", "#f5efde")]].concat();
    assert!(first_words(&validate_state_distinctness(&state(&collapsed))).contains(&"listActiveVsHover"));
    assert!(STATE_DISTINCTNESS_PAIRS
        .iter()
        .all(|pair| !pair.is_sibling || pair.surface.is_some()));
    assert_eq!(
        first_words(&validate_state_distinctness(&state(&[("app.shadow", "#ffffff00")]))),
        ["appShadow"]
    );
    assert!(validate_state_distinctness(&state(&[("app.shadow", "#00000026")])).is_empty());
    assert!(APP_SHADOW_MIN_ALPHA < 1.0);
    let flat_tabs = [
        ("tabBar.tabActiveBackground", "#181825"),
        ("tabBar.tabInactiveBackground", "#181825"),
    ];
    assert!(validate_state_distinctness(&state(&flat_tabs)).is_empty());
    let collapsed_tabs = [flat_tabs.as_slice(), &[("tabBar.tabActiveIndicator", "#181825")]].concat();
    assert_eq!(
        first_words(&validate_state_distinctness(&state(&collapsed_tabs))),
        ["tabActive", "tabActiveVsInactive"]
    );
}

#[test]
fn 상태_구별성_수리는_ts_사례와_같은_값과_문구를_낸다() {
    let (colors, repairs) = repaired_state(state(&[("editor.selection", "#1e1e2e")]));
    assert_eq!(colors["editor.background"], "#1e1e2e");
    assert_ne!(colors["editor.selection"], "#1e1e2e");
    let distance = delta_e76(&colors["editor.selection"], "#1e1e2e").unwrap();
    assert!(distance >= STATE_MIN_DISTINCT_DELTA_E);
    assert!(distance < STATE_MIN_DISTINCT_DELTA_E + STATE_REPAIR_DELTA_E_HEADROOM);
    assert_eq!(repairs, ["editor.selection: #1e1e2e -> #232333 (editorSelection 구별성 확보)"]);

    let (first, _) = repaired_state(state(&[("terminal.selection", "#1e1e2e")]));
    let (again, _) = repaired_state(state(&[("terminal.selection", "#1e1e2e")]));
    assert_eq!(again, first);
    let (_, second_repairs) = repaired_state(first.clone());
    assert!(second_repairs.is_empty());
    assert!(validate_state_distinctness(&first).is_empty());

    let (colors, _) = repaired_state(state(&[
        ("tabBar.tabActiveBackground", "#181825"),
        ("tabBar.tabInactiveBackground", "#181825"),
        ("tabBar.tabActiveIndicator", "#181825"),
    ]));
    assert_eq!(colors["tabBar.tabActiveBackground"], "#181825");
    assert_ne!(colors["tabBar.tabActiveIndicator"], "#181825");
    assert!(validate_state_distinctness(&colors).is_empty());

    let (colors, _) = repaired_state(state(&[
        ("input.border", "#1e1e2e"),
        ("panel.background", "#1e1e2e"),
        ("input.background", "#1e1e2e"),
    ]));
    assert!(validate_state_distinctness(&colors).is_empty());

    let (colors, _) = repaired_state(state(AYU_SIBLING_OVERRIDES));
    assert_eq!(colors["explorer.itemHover"], "#47526640");
    assert!(validate_state_distinctness(&colors).is_empty());

    let (colors, repairs) = repaired_state(state(&[("app.shadow", "#ffffff00")]));
    assert_eq!(colors["app.shadow"], "#00000026");
    assert_eq!(repairs, ["app.shadow: #ffffff00 -> #00000026 (appShadow 구별성 확보)"]);
    assert!(repaired_state(colors).1.is_empty());

    let (colors, repairs) = repaired_state(state(&[("app.foreground", "transparent"), ("editor.selection", "#1e1e2e")]));
    assert_eq!(colors["editor.selection"], "#1e1e2e");
    assert!(repairs.is_empty());
}

#[test]
fn 출력_대비_검증과_쌍_수리는_ts_사례와_같다() {
    let translucent = contrast_colors(&[("panel.matchHighlight", "#ffd33d22")]);
    assert!(validate_output_colors(&translucent)
        .iter()
        .any(|error| error.contains("matchHighlight")));
    assert!(!validate_output_colors(&contrast_colors(&[("panel.matchHighlight", "#ffd33d")]))
        .iter()
        .any(|error| error.contains("matchHighlight")));

    let (colors, repairs) = repaired_contrast(translucent, &[("editor.foreground", "#d4d4d4")]);
    assert_eq!(colors["panel.matchHighlight"], "#d4d4d4");
    assert!(repairs.iter().any(|repair| repair.contains("matchHighlight")));

    let (colors, repairs) = repaired_contrast(
        contrast_colors(&[("app.foreground", "#000000"), ("panel.matchHighlight", "#569cd6")]),
        &[("editor.foreground", "#d4d4d4")],
    );
    assert_eq!(colors["app.foreground"], "#d4d4d4");
    assert!(repairs.iter().any(|repair| repair.starts_with("app.foreground")));
    assert_eq!(colors["panel.matchHighlight"], "#569cd6");

    assert!(validate_output_colors(&contrast_colors(&[
        ("app.foreground", "#00000030"),
        ("app.background", "#ffffff"),
        ("panel.matchHighlight", "#d4d4d4"),
    ]))
    .iter()
    .any(|error| error.starts_with("app 대비 부족")));
    assert!(validate_output_colors(&contrast_colors(&[
        ("panel.matchHighlight", "#569cd6"),
        ("list.activeBackground", "#569cd6"),
    ]))
    .is_empty());
    assert!(
        validate_output_colors(&contrast_colors(&[("app.foreground", "#101010"), ("app.background", "#0e0e0e"),]))
            .iter()
            .any(|error| error.starts_with("app 대비 부족"))
    );

    let (colors, repairs) = repaired_contrast(
        contrast_colors(&[("list.foreground", "#3a3a3a")]),
        &[("editor.foreground", "#d4d4d4")],
    );
    assert_eq!(colors["list.foreground"], "#d4d4d4");
    assert!(repairs.iter().any(|repair| repair.starts_with("list.foreground")));

    let (colors, _) = repaired_contrast(
        contrast_colors(&[("panel.matchHighlight", "#ffd33d22"), ("list.activeBackground", "#606060")]),
        &[("textLink.foreground", "#0d0d0d"), ("editor.foreground", "#7f7f7f")],
    );
    assert_eq!(colors["panel.matchHighlight"], "#7f7f7f");
    assert!(validate_output_colors(&colors).is_empty());

    let (colors, repairs) = repaired_contrast(
        contrast_colors(&[
            ("app.foreground", "#575279"),
            ("app.background", "#faf4ed"),
            ("editor.foreground", "#575279"),
            ("editor.background", "#faf4ed"),
            ("panel.sectionHeader", "#575279"),
            ("panel.background", "#faf4ed"),
            ("tooltip.background", "#faf4ed"),
            ("panel.matchHighlight", "#907aa9"),
            ("list.activeBackground", "#ADD6FF"),
        ]),
        &[("textLink.foreground", "#907aa9"), ("editor.foreground", "#575279")],
    );
    assert_eq!(colors["panel.matchHighlight"], "#907aa9");
    assert!(!repairs.iter().any(|repair| repair.starts_with("panel.matchHighlight")));
    assert!(validate_output_colors(&colors).is_empty());

    let (colors, repairs) = repaired_contrast(
        contrast_colors(&[
            ("list.foreground", "#d8dee9"),
            ("list.background", "#2e3440"),
            ("list.hoverBackground", "#3b4252"),
            ("list.activeBackground", "#88c0d0"),
        ]),
        &[("editor.foreground", "#2e3440")],
    );
    assert_eq!(colors["list.foreground"], "#d8dee9");
    assert!(!repairs.iter().any(|repair| repair.starts_with("list.foreground")));
}

#[test]
fn 컴포넌트_대비_수리는_ts_사례와_같다() {
    let (colors, repairs) = repaired_components(colors_with(
        &[
            ("app.background", "#ffffff"),
            ("explorer.background", "#f6f8fa"),
            ("explorer.gitAdded", "#28a745"),
        ],
        &[],
    ));
    let repaired = &colors["explorer.gitAdded"];
    assert!(repairs.iter().any(|repair| repair.starts_with("explorer.gitAdded")));
    assert!(contrast_ratio(repaired, "#f6f8fa").unwrap() >= MIN_COMPONENT_CONTRAST);
    assert!(delta_e76(repaired, "#28a745").unwrap() < COMPONENT_REPAIR_MAX_DELTA_E);

    let (_, repairs) = repaired_components(colors_with(
        &[
            ("app.background", "#282828"),
            ("appSidebar.background", "#282828"),
            ("panel.background", "#1e1f1c"),
            ("statusIndicator.error", "#cc241d"),
        ],
        &[],
    ));
    assert_eq!(
        repairs.iter().filter(|repair| repair.starts_with("statusIndicator.error")).count(),
        1
    );
    assert!(repairs[0].contains("statusBarError/problemError"));

    let (colors, repairs) = repaired_components(colors_with(
        &[
            ("app.background", "#ffffff"),
            ("appSidebar.background", "#9d9d9d"),
            ("panel.background", "#333333"),
            ("statusIndicator.error", "#ff0000"),
        ],
        &[],
    ));
    assert!(repairs.is_empty());
    assert_eq!(colors["statusIndicator.error"], "#ff0000");

    let (colors, repairs) = repaired_components(colors_with(
        &[
            ("app.background", "#000000"),
            ("list.foreground", "#888888"),
            ("list.background", "#000000"),
            ("list.hoverBackground", "#777777"),
            ("list.activeBackground", "#ffffff"),
        ],
        &[],
    ));
    assert!(repairs.is_empty());
    assert_eq!(colors["list.foreground"], "#888888");

    let (colors, repairs) = repaired_components(colors_with(
        &[
            ("app.background", "#ffffff"),
            ("list.foreground", "#797593"),
            ("list.background", "#ffffff"),
            ("list.hoverBackground", "#ffffff"),
            ("list.activeBackground", "#6e6a86"),
        ],
        &[],
    ));
    assert!(repairs.is_empty());
    assert_eq!(colors["list.foreground"], "#797593");

    let (colors, repairs) = repaired_components(colors_with(ACCENT_MENU_COLORS, &[]));
    assert_eq!(colors["app.foreground"], "#3b3b3b");
    assert_eq!(colors["menu.itemHover"], "#f1f1f1");
    assert_eq!(repairs, ["menu.itemHover: #005fb8 -> #f1f1f1 (menuItemHoverText 대비 확보)"]);

    let (colors, repairs) = repaired_components(colors_with(ACCENT_MENU_COLORS, &[("list.hoverBackground", "#ffffff")]));
    assert_eq!(colors["menu.itemHover"], "#005fb8");
    assert!(repairs.is_empty());

    let (colors, repairs) = repaired_components(colors_with(
        &[
            ("app.background", "#0d1017"),
            ("app.foreground", "#5a6378"),
            ("menu.background", "#0f131a"),
            ("menu.itemHover", "#47526633"),
            ("list.hoverBackground", "#47526640"),
        ],
        &[],
    ));
    assert_eq!(colors["menu.itemHover"], "#47526633");
    assert!(repairs.is_empty());
}
