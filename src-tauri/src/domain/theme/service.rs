use std::collections::BTreeMap;

use crate::domain::theme::types::{ResolvedTheme, SyntaxStyle, Theme, ThemeSummary, ThemeType, THEME_SCHEMA_VERSION};
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::paths::AppPaths;

pub const BUILTIN_DARK_ID: &str = "taide-dark";
pub const BUILTIN_LIGHT_ID: &str = "taide-light";

const COLOR_NAMESPACES: &[(&str, &[&str])] = &[
    ("app", &["background", "foreground", "border", "focusBorder", "shadow", "accent"]),
    (
        "appSidebar",
        &["background", "itemHover", "itemActive", "iconDefault", "iconAgentRunning", "badge"],
    ),
    (
        "tabBar",
        &[
            "background",
            "tabActiveBackground",
            "tabInactiveBackground",
            "tabActiveForeground",
            "tabInactiveForeground",
            "tabBorder",
            "tabActiveIndicator",
            "dirtyDot",
            "previewForeground",
            "dropTarget",
        ],
    ),
    (
        "explorer",
        &[
            "background",
            "itemHover",
            "itemSelected",
            "itemFocused",
            "indentGuide",
            "folderIcon",
            "gitModified",
            "gitAdded",
            "gitDeleted",
            "gitUntracked",
            "gitIgnored",
        ],
    ),
    (
        "panel",
        &["background", "sectionHeader", "inputBackground", "inputBorder", "matchHighlight"],
    ),
    (
        "editor",
        &[
            "background",
            "foreground",
            "lineHighlight",
            "cursor",
            "selection",
            "inactiveSelection",
            "lineNumber",
            "lineNumberActive",
            "indentGuide",
            "whitespace",
            "bracketMatch",
            "findMatch",
            "findMatchHighlight",
            "hoverBackground",
            "widgetBackground",
            "widgetBorder",
        ],
    ),
    ("editorGutter", &["addedBackground", "modifiedBackground", "deletedBackground"]),
    ("editorBlame", &["foreground", "background"]),
    (
        "diff",
        &[
            "insertedBackground",
            "insertedLineBackground",
            "removedBackground",
            "removedLineBackground",
            "border",
        ],
    ),
    (
        "terminal",
        &[
            "background",
            "foreground",
            "cursor",
            "selection",
            "commandBlockBorder",
            "linkForeground",
        ],
    ),
    (
        "git",
        &["added", "modified", "deleted", "renamed", "untracked", "conflicted", "staged"],
    ),
    (
        "graph",
        &[
            "lane1",
            "lane2",
            "lane3",
            "lane4",
            "lane5",
            "lane6",
            "lane7",
            "lane8",
            "lane9",
            "lane10",
            "lane11",
            "lane12",
            "refBranch",
            "refTag",
            "refHead",
        ],
    ),
    ("statusIndicator", &["info", "warning", "error", "success"]),
    ("menu", &["background", "border", "itemHover", "separator"]),
    ("popover", &["background", "border", "itemHover", "separator"]),
    ("tooltip", &["background", "border", "itemHover", "separator"]),
    ("modal", &["background", "border", "itemHover", "separator"]),
    ("scrollbar", &["thumb", "thumbHover", "track"]),
    ("input", &["background", "foreground", "border", "placeholder", "focusBorder"]),
    (
        "button",
        &[
            "background",
            "foreground",
            "hoverBackground",
            "primaryBackground",
            "primaryForeground",
        ],
    ),
    ("list", &["background", "hoverBackground", "activeBackground", "foreground"]),
];

const SYNTAX_TOKENS: &[&str] = &[
    "keyword",
    "storage",
    "operator",
    "string",
    "number",
    "regexp",
    "comment",
    "docComment",
    "function",
    "method",
    "variable",
    "parameter",
    "property",
    "type",
    "class",
    "interface",
    "enum",
    "constant",
    "namespace",
    "decorator",
    "tag",
    "attribute",
    "punctuation",
    "invalid",
    "link",
    "markdownHeading",
    "markdownEmphasis",
    "markdownStrong",
    "markdownCode",
    "markdownQuote",
    "markdownListMarker",
];

const TERMINAL_ANSI_TOKENS: &[&str] = &[
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "brightBlack",
    "brightRed",
    "brightGreen",
    "brightYellow",
    "brightBlue",
    "brightMagenta",
    "brightCyan",
    "brightWhite",
    "background",
    "foreground",
    "cursor",
    "selection",
];

pub fn required_color_keys() -> Vec<String> {
    COLOR_NAMESPACES
        .iter()
        .flat_map(|(namespace, tokens)| tokens.iter().map(move |token| format!("{namespace}.{token}")))
        .collect()
}

pub fn required_syntax_keys() -> Vec<&'static str> {
    SYNTAX_TOKENS.to_vec()
}

pub fn required_terminal_keys() -> Vec<&'static str> {
    TERMINAL_ANSI_TOKENS.to_vec()
}

fn map_from_pairs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
}

fn syntax_from_pairs(pairs: &[(&str, &str, bool, bool)]) -> BTreeMap<String, SyntaxStyle> {
    pairs
        .iter()
        .map(|(key, fg, bold, italic)| {
            (
                key.to_string(),
                SyntaxStyle {
                    fg: fg.to_string(),
                    bold: *bold,
                    italic: *italic,
                },
            )
        })
        .collect()
}

fn dark_colors() -> BTreeMap<String, String> {
    map_from_pairs(&[
        ("app.background", "#1e1e2e"),
        ("app.foreground", "#cdd6f4"),
        ("app.border", "#313244"),
        ("app.focusBorder", "#89b4fa"),
        ("app.shadow", "#00000066"),
        ("app.accent", "#89b4fa"),
        ("appSidebar.background", "#181825"),
        ("appSidebar.itemHover", "#313244"),
        ("appSidebar.itemActive", "#45475a"),
        ("appSidebar.iconDefault", "#a6adc8"),
        ("appSidebar.iconAgentRunning", "#a6e3a1"),
        ("appSidebar.badge", "#f38ba8"),
        ("tabBar.background", "#181825"),
        ("tabBar.tabActiveBackground", "#1e1e2e"),
        ("tabBar.tabInactiveBackground", "#181825"),
        ("tabBar.tabActiveForeground", "#cdd6f4"),
        ("tabBar.tabInactiveForeground", "#7f849c"),
        ("tabBar.tabBorder", "#313244"),
        ("tabBar.tabActiveIndicator", "#89b4fa"),
        ("tabBar.dirtyDot", "#f9e2af"),
        ("tabBar.previewForeground", "#7f849c"),
        ("tabBar.dropTarget", "#89b4fa"),
        ("explorer.background", "#181825"),
        ("explorer.itemHover", "#313244"),
        ("explorer.itemSelected", "#45475a"),
        ("explorer.itemFocused", "#585b70"),
        ("explorer.indentGuide", "#313244"),
        ("explorer.folderIcon", "#89b4fa"),
        ("explorer.gitModified", "#f9e2af"),
        ("explorer.gitAdded", "#a6e3a1"),
        ("explorer.gitDeleted", "#f38ba8"),
        ("explorer.gitUntracked", "#94e2d5"),
        ("explorer.gitIgnored", "#6c7086"),
        ("panel.background", "#181825"),
        ("panel.sectionHeader", "#a6adc8"),
        ("panel.inputBackground", "#1e1e2e"),
        ("panel.inputBorder", "#313244"),
        ("panel.matchHighlight", "#f9e2af"),
        ("editor.background", "#1e1e2e"),
        ("editor.foreground", "#cdd6f4"),
        ("editor.lineHighlight", "#313244"),
        ("editor.cursor", "#f5e0dc"),
        ("editor.selection", "#45475a"),
        ("editor.inactiveSelection", "#313244"),
        ("editor.lineNumber", "#6c7086"),
        ("editor.lineNumberActive", "#cdd6f4"),
        ("editor.indentGuide", "#313244"),
        ("editor.whitespace", "#45475a"),
        ("editor.bracketMatch", "#cba6f7"),
        ("editor.findMatch", "#fab387"),
        ("editor.findMatchHighlight", "#fab38766"),
        ("editor.hoverBackground", "#181825"),
        ("editor.widgetBackground", "#181825"),
        ("editor.widgetBorder", "#313244"),
        ("editorGutter.addedBackground", "#a6e3a1"),
        ("editorGutter.modifiedBackground", "#89b4fa"),
        ("editorGutter.deletedBackground", "#f38ba8"),
        ("editorBlame.foreground", "#6c7086"),
        ("editorBlame.background", "transparent"),
        ("diff.insertedBackground", "#a6e3a133"),
        ("diff.insertedLineBackground", "#a6e3a11a"),
        ("diff.removedBackground", "#f38ba833"),
        ("diff.removedLineBackground", "#f38ba81a"),
        ("diff.border", "#313244"),
        ("terminal.background", "#1e1e2e"),
        ("terminal.foreground", "#cdd6f4"),
        ("terminal.cursor", "#f5e0dc"),
        ("terminal.selection", "#45475a"),
        ("terminal.commandBlockBorder", "#313244"),
        ("terminal.linkForeground", "#89b4fa"),
        ("git.added", "#a6e3a1"),
        ("git.modified", "#f9e2af"),
        ("git.deleted", "#f38ba8"),
        ("git.renamed", "#74c7ec"),
        ("git.untracked", "#94e2d5"),
        ("git.conflicted", "#fab387"),
        ("git.staged", "#cba6f7"),
        ("graph.lane1", "#f5e0dc"),
        ("graph.lane2", "#f2cdcd"),
        ("graph.lane3", "#f5c2e7"),
        ("graph.lane4", "#cba6f7"),
        ("graph.lane5", "#f38ba8"),
        ("graph.lane6", "#eba0ac"),
        ("graph.lane7", "#fab387"),
        ("graph.lane8", "#f9e2af"),
        ("graph.lane9", "#a6e3a1"),
        ("graph.lane10", "#94e2d5"),
        ("graph.lane11", "#89dceb"),
        ("graph.lane12", "#74c7ec"),
        ("graph.refBranch", "#89b4fa"),
        ("graph.refTag", "#f9e2af"),
        ("graph.refHead", "#a6e3a1"),
        ("statusIndicator.info", "#89b4fa"),
        ("statusIndicator.warning", "#f9e2af"),
        ("statusIndicator.error", "#f38ba8"),
        ("statusIndicator.success", "#a6e3a1"),
        ("menu.background", "#181825"),
        ("menu.border", "#313244"),
        ("menu.itemHover", "#45475a"),
        ("menu.separator", "#313244"),
        ("popover.background", "#181825"),
        ("popover.border", "#313244"),
        ("popover.itemHover", "#45475a"),
        ("popover.separator", "#313244"),
        ("tooltip.background", "#181825"),
        ("tooltip.border", "#313244"),
        ("tooltip.itemHover", "#45475a"),
        ("tooltip.separator", "#313244"),
        ("modal.background", "#181825"),
        ("modal.border", "#313244"),
        ("modal.itemHover", "#45475a"),
        ("modal.separator", "#313244"),
        ("scrollbar.thumb", "#45475a"),
        ("scrollbar.thumbHover", "#585b70"),
        ("scrollbar.track", "transparent"),
        ("input.background", "#1e1e2e"),
        ("input.foreground", "#cdd6f4"),
        ("input.border", "#313244"),
        ("input.placeholder", "#6c7086"),
        ("input.focusBorder", "#89b4fa"),
        ("button.background", "#313244"),
        ("button.foreground", "#cdd6f4"),
        ("button.hoverBackground", "#45475a"),
        ("button.primaryBackground", "#89b4fa"),
        ("button.primaryForeground", "#1e1e2e"),
        ("list.background", "#181825"),
        ("list.hoverBackground", "#313244"),
        ("list.activeBackground", "#45475a"),
        ("list.foreground", "#cdd6f4"),
    ])
}

fn light_colors() -> BTreeMap<String, String> {
    map_from_pairs(&[
        ("app.background", "#eff1f5"),
        ("app.foreground", "#4c4f69"),
        ("app.border", "#ccd0da"),
        ("app.focusBorder", "#1e66f5"),
        ("app.shadow", "#00000022"),
        ("app.accent", "#1e66f5"),
        ("appSidebar.background", "#e6e9ef"),
        ("appSidebar.itemHover", "#ccd0da"),
        ("appSidebar.itemActive", "#bcc0cc"),
        ("appSidebar.iconDefault", "#6c6f85"),
        ("appSidebar.iconAgentRunning", "#40a02b"),
        ("appSidebar.badge", "#d20f39"),
        ("tabBar.background", "#e6e9ef"),
        ("tabBar.tabActiveBackground", "#eff1f5"),
        ("tabBar.tabInactiveBackground", "#e6e9ef"),
        ("tabBar.tabActiveForeground", "#4c4f69"),
        ("tabBar.tabInactiveForeground", "#8c8fa1"),
        ("tabBar.tabBorder", "#ccd0da"),
        ("tabBar.tabActiveIndicator", "#1e66f5"),
        ("tabBar.dirtyDot", "#df8e1d"),
        ("tabBar.previewForeground", "#8c8fa1"),
        ("tabBar.dropTarget", "#1e66f5"),
        ("explorer.background", "#e6e9ef"),
        ("explorer.itemHover", "#ccd0da"),
        ("explorer.itemSelected", "#bcc0cc"),
        ("explorer.itemFocused", "#acb0be"),
        ("explorer.indentGuide", "#ccd0da"),
        ("explorer.folderIcon", "#1e66f5"),
        ("explorer.gitModified", "#df8e1d"),
        ("explorer.gitAdded", "#40a02b"),
        ("explorer.gitDeleted", "#d20f39"),
        ("explorer.gitUntracked", "#179299"),
        ("explorer.gitIgnored", "#9ca0b0"),
        ("panel.background", "#e6e9ef"),
        ("panel.sectionHeader", "#6c6f85"),
        ("panel.inputBackground", "#eff1f5"),
        ("panel.inputBorder", "#ccd0da"),
        ("panel.matchHighlight", "#df8e1d"),
        ("editor.background", "#eff1f5"),
        ("editor.foreground", "#4c4f69"),
        ("editor.lineHighlight", "#ccd0da"),
        ("editor.cursor", "#dc8a78"),
        ("editor.selection", "#bcc0cc"),
        ("editor.inactiveSelection", "#ccd0da"),
        ("editor.lineNumber", "#9ca0b0"),
        ("editor.lineNumberActive", "#4c4f69"),
        ("editor.indentGuide", "#ccd0da"),
        ("editor.whitespace", "#bcc0cc"),
        ("editor.bracketMatch", "#8839ef"),
        ("editor.findMatch", "#fe640b"),
        ("editor.findMatchHighlight", "#fe640b33"),
        ("editor.hoverBackground", "#e6e9ef"),
        ("editor.widgetBackground", "#e6e9ef"),
        ("editor.widgetBorder", "#ccd0da"),
        ("editorGutter.addedBackground", "#40a02b"),
        ("editorGutter.modifiedBackground", "#1e66f5"),
        ("editorGutter.deletedBackground", "#d20f39"),
        ("editorBlame.foreground", "#9ca0b0"),
        ("editorBlame.background", "transparent"),
        ("diff.insertedBackground", "#40a02b33"),
        ("diff.insertedLineBackground", "#40a02b1a"),
        ("diff.removedBackground", "#d20f3933"),
        ("diff.removedLineBackground", "#d20f391a"),
        ("diff.border", "#ccd0da"),
        ("terminal.background", "#eff1f5"),
        ("terminal.foreground", "#4c4f69"),
        ("terminal.cursor", "#dc8a78"),
        ("terminal.selection", "#bcc0cc"),
        ("terminal.commandBlockBorder", "#ccd0da"),
        ("terminal.linkForeground", "#1e66f5"),
        ("git.added", "#40a02b"),
        ("git.modified", "#df8e1d"),
        ("git.deleted", "#d20f39"),
        ("git.renamed", "#209fb5"),
        ("git.untracked", "#179299"),
        ("git.conflicted", "#fe640b"),
        ("git.staged", "#8839ef"),
        ("graph.lane1", "#dc8a78"),
        ("graph.lane2", "#dd7878"),
        ("graph.lane3", "#ea76cb"),
        ("graph.lane4", "#8839ef"),
        ("graph.lane5", "#d20f39"),
        ("graph.lane6", "#e64553"),
        ("graph.lane7", "#fe640b"),
        ("graph.lane8", "#df8e1d"),
        ("graph.lane9", "#40a02b"),
        ("graph.lane10", "#179299"),
        ("graph.lane11", "#04a5e5"),
        ("graph.lane12", "#209fb5"),
        ("graph.refBranch", "#1e66f5"),
        ("graph.refTag", "#df8e1d"),
        ("graph.refHead", "#40a02b"),
        ("statusIndicator.info", "#1e66f5"),
        ("statusIndicator.warning", "#df8e1d"),
        ("statusIndicator.error", "#d20f39"),
        ("statusIndicator.success", "#40a02b"),
        ("menu.background", "#e6e9ef"),
        ("menu.border", "#ccd0da"),
        ("menu.itemHover", "#bcc0cc"),
        ("menu.separator", "#ccd0da"),
        ("popover.background", "#e6e9ef"),
        ("popover.border", "#ccd0da"),
        ("popover.itemHover", "#bcc0cc"),
        ("popover.separator", "#ccd0da"),
        ("tooltip.background", "#e6e9ef"),
        ("tooltip.border", "#ccd0da"),
        ("tooltip.itemHover", "#bcc0cc"),
        ("tooltip.separator", "#ccd0da"),
        ("modal.background", "#e6e9ef"),
        ("modal.border", "#ccd0da"),
        ("modal.itemHover", "#bcc0cc"),
        ("modal.separator", "#ccd0da"),
        ("scrollbar.thumb", "#bcc0cc"),
        ("scrollbar.thumbHover", "#acb0be"),
        ("scrollbar.track", "transparent"),
        ("input.background", "#eff1f5"),
        ("input.foreground", "#4c4f69"),
        ("input.border", "#ccd0da"),
        ("input.placeholder", "#9ca0b0"),
        ("input.focusBorder", "#1e66f5"),
        ("button.background", "#ccd0da"),
        ("button.foreground", "#4c4f69"),
        ("button.hoverBackground", "#bcc0cc"),
        ("button.primaryBackground", "#1e66f5"),
        ("button.primaryForeground", "#eff1f5"),
        ("list.background", "#e6e9ef"),
        ("list.hoverBackground", "#ccd0da"),
        ("list.activeBackground", "#bcc0cc"),
        ("list.foreground", "#4c4f69"),
    ])
}

fn dark_syntax() -> BTreeMap<String, SyntaxStyle> {
    syntax_from_pairs(&[
        ("keyword", "#cba6f7", false, false),
        ("storage", "#cba6f7", false, false),
        ("operator", "#89dceb", false, false),
        ("string", "#a6e3a1", false, false),
        ("number", "#fab387", false, false),
        ("regexp", "#f38ba8", false, false),
        ("comment", "#6c7086", false, true),
        ("docComment", "#6c7086", false, true),
        ("function", "#89b4fa", false, false),
        ("method", "#89b4fa", false, false),
        ("variable", "#cdd6f4", false, false),
        ("parameter", "#eba0ac", false, true),
        ("property", "#b4befe", false, false),
        ("type", "#f9e2af", false, false),
        ("class", "#f9e2af", true, false),
        ("interface", "#f9e2af", false, true),
        ("enum", "#94e2d5", false, false),
        ("constant", "#fab387", false, false),
        ("namespace", "#74c7ec", false, false),
        ("decorator", "#f5c2e7", false, false),
        ("tag", "#f38ba8", false, false),
        ("attribute", "#f9e2af", false, false),
        ("punctuation", "#9399b2", false, false),
        ("invalid", "#f38ba8", true, false),
        ("link", "#74c7ec", false, false),
        ("markdownHeading", "#89b4fa", true, false),
        ("markdownEmphasis", "#cdd6f4", false, true),
        ("markdownStrong", "#cdd6f4", true, false),
        ("markdownCode", "#a6e3a1", false, false),
        ("markdownQuote", "#7f849c", false, true),
        ("markdownListMarker", "#cba6f7", false, false),
    ])
}

fn light_syntax() -> BTreeMap<String, SyntaxStyle> {
    syntax_from_pairs(&[
        ("keyword", "#8839ef", false, false),
        ("storage", "#8839ef", false, false),
        ("operator", "#04a5e5", false, false),
        ("string", "#40a02b", false, false),
        ("number", "#fe640b", false, false),
        ("regexp", "#d20f39", false, false),
        ("comment", "#9ca0b0", false, true),
        ("docComment", "#9ca0b0", false, true),
        ("function", "#1e66f5", false, false),
        ("method", "#1e66f5", false, false),
        ("variable", "#4c4f69", false, false),
        ("parameter", "#e64553", false, true),
        ("property", "#7287fd", false, false),
        ("type", "#df8e1d", false, false),
        ("class", "#df8e1d", true, false),
        ("interface", "#df8e1d", false, true),
        ("enum", "#179299", false, false),
        ("constant", "#fe640b", false, false),
        ("namespace", "#209fb5", false, false),
        ("decorator", "#ea76cb", false, false),
        ("tag", "#d20f39", false, false),
        ("attribute", "#df8e1d", false, false),
        ("punctuation", "#7c7f93", false, false),
        ("invalid", "#d20f39", true, false),
        ("link", "#209fb5", false, false),
        ("markdownHeading", "#1e66f5", true, false),
        ("markdownEmphasis", "#4c4f69", false, true),
        ("markdownStrong", "#4c4f69", true, false),
        ("markdownCode", "#40a02b", false, false),
        ("markdownQuote", "#8c8fa1", false, true),
        ("markdownListMarker", "#8839ef", false, false),
    ])
}

fn dark_terminal() -> BTreeMap<String, String> {
    map_from_pairs(&[
        ("black", "#45475a"),
        ("red", "#f38ba8"),
        ("green", "#a6e3a1"),
        ("yellow", "#f9e2af"),
        ("blue", "#89b4fa"),
        ("magenta", "#f5c2e7"),
        ("cyan", "#94e2d5"),
        ("white", "#bac2de"),
        ("brightBlack", "#585b70"),
        ("brightRed", "#f38ba8"),
        ("brightGreen", "#a6e3a1"),
        ("brightYellow", "#f9e2af"),
        ("brightBlue", "#89b4fa"),
        ("brightMagenta", "#f5c2e7"),
        ("brightCyan", "#94e2d5"),
        ("brightWhite", "#a6adc8"),
        ("background", "#1e1e2e"),
        ("foreground", "#cdd6f4"),
        ("cursor", "#f5e0dc"),
        ("selection", "#585b70"),
    ])
}

fn light_terminal() -> BTreeMap<String, String> {
    map_from_pairs(&[
        ("black", "#5c5f77"),
        ("red", "#d20f39"),
        ("green", "#40a02b"),
        ("yellow", "#df8e1d"),
        ("blue", "#1e66f5"),
        ("magenta", "#ea76cb"),
        ("cyan", "#179299"),
        ("white", "#acb0be"),
        ("brightBlack", "#6c6f85"),
        ("brightRed", "#d20f39"),
        ("brightGreen", "#40a02b"),
        ("brightYellow", "#df8e1d"),
        ("brightBlue", "#1e66f5"),
        ("brightMagenta", "#ea76cb"),
        ("brightCyan", "#179299"),
        ("brightWhite", "#bcc0cc"),
        ("background", "#eff1f5"),
        ("foreground", "#4c4f69"),
        ("cursor", "#dc8a78"),
        ("selection", "#acb0be"),
    ])
}

pub fn builtin_dark() -> Theme {
    Theme {
        version: THEME_SCHEMA_VERSION,
        id: BUILTIN_DARK_ID.to_string(),
        name: "TAIDE Dark".to_string(),
        theme_type: ThemeType::Dark,
        extends: None,
        palette: BTreeMap::new(),
        colors: dark_colors(),
        syntax: dark_syntax(),
        terminal: dark_terminal(),
    }
}

pub fn builtin_light() -> Theme {
    Theme {
        version: THEME_SCHEMA_VERSION,
        id: BUILTIN_LIGHT_ID.to_string(),
        name: "TAIDE Light".to_string(),
        theme_type: ThemeType::Light,
        extends: None,
        palette: BTreeMap::new(),
        colors: light_colors(),
        syntax: light_syntax(),
        terminal: light_terminal(),
    }
}

pub fn builtin_by_id(theme_id: &str) -> Option<Theme> {
    match theme_id {
        BUILTIN_DARK_ID => Some(builtin_dark()),
        BUILTIN_LIGHT_ID => Some(builtin_light()),
        _ => None,
    }
}

fn matching_builtin(theme_type: ThemeType) -> Theme {
    match theme_type {
        ThemeType::Dark => builtin_dark(),
        ThemeType::Light => builtin_light(),
    }
}

fn summarize(theme: &Theme, builtin: bool) -> ThemeSummary {
    ThemeSummary {
        id: theme.id.clone(),
        name: theme.name.clone(),
        theme_type: theme.theme_type,
        builtin,
    }
}

fn resolve_value(value: &str, palette: &BTreeMap<String, String>, warnings: &mut Vec<String>, context: &str) -> String {
    let Some(key) = value.strip_prefix('$') else {
        return value.to_string();
    };
    match palette.get(key) {
        Some(resolved) => resolved.clone(),
        None => {
            warnings.push(format!("unresolved palette reference '${key}' in {context}"));
            value.to_string()
        }
    }
}

fn resolve_colors(theme: &Theme, warnings: &mut Vec<String>) -> BTreeMap<String, String> {
    theme
        .colors
        .iter()
        .map(|(key, value)| (key.clone(), resolve_value(value, &theme.palette, warnings, key)))
        .collect()
}

fn resolve_syntax(theme: &Theme, warnings: &mut Vec<String>) -> BTreeMap<String, SyntaxStyle> {
    theme
        .syntax
        .iter()
        .map(|(key, style)| {
            let fg = resolve_value(&style.fg, &theme.palette, warnings, key);
            (
                key.clone(),
                SyntaxStyle {
                    fg,
                    bold: style.bold,
                    italic: style.italic,
                },
            )
        })
        .collect()
}

fn resolve_terminal(theme: &Theme, warnings: &mut Vec<String>) -> BTreeMap<String, String> {
    theme
        .terminal
        .iter()
        .map(|(key, value)| (key.clone(), resolve_value(value, &theme.palette, warnings, key)))
        .collect()
}

pub fn resolve_theme(theme: &Theme, base: Option<&Theme>) -> ResolvedTheme {
    let mut warnings = Vec::new();
    let mut colors = resolve_colors(theme, &mut warnings);
    let mut syntax = resolve_syntax(theme, &mut warnings);
    let mut terminal = resolve_terminal(theme, &mut warnings);

    if let Some(base_theme) = base {
        let mut base_warnings = Vec::new();
        let base_colors = resolve_colors(base_theme, &mut base_warnings);
        let base_syntax = resolve_syntax(base_theme, &mut base_warnings);
        let base_terminal = resolve_terminal(base_theme, &mut base_warnings);

        for (key, value) in base_colors {
            colors.entry(key.clone()).or_insert_with(|| {
                warnings.push(format!("colors.{key} filled from base theme '{}'", base_theme.id));
                value
            });
        }
        for (key, value) in base_syntax {
            syntax.entry(key.clone()).or_insert_with(|| {
                warnings.push(format!("syntax.{key} filled from base theme '{}'", base_theme.id));
                value
            });
        }
        for (key, value) in base_terminal {
            terminal.entry(key.clone()).or_insert_with(|| {
                warnings.push(format!("terminal.{key} filled from base theme '{}'", base_theme.id));
                value
            });
        }
    }

    ResolvedTheme {
        id: theme.id.clone(),
        name: theme.name.clone(),
        theme_type: theme.theme_type,
        colors,
        syntax,
        terminal,
        warnings,
    }
}

pub fn list_themes(paths: &AppPaths) -> Vec<ThemeSummary> {
    let mut list = vec![summarize(&builtin_dark(), true), summarize(&builtin_light(), true)];

    let Ok(entries) = std::fs::read_dir(paths.themes_dir()) else {
        return list;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if let Ok(Some(theme)) = persist::read_json::<Theme>(&path) {
            list.push(summarize(&theme, false));
        }
    }

    list
}

pub fn load_theme(paths: &AppPaths, theme_id: &str) -> AppResult<ResolvedTheme> {
    if let Some(theme) = builtin_by_id(theme_id) {
        return Ok(resolve_theme(&theme, None));
    }

    let path = paths.themes_dir().join(format!("{theme_id}.json"));
    let theme: Theme = persist::read_json(&path)?.ok_or_else(|| AppError::NotFound(format!("theme not found: {theme_id}")))?;

    let base = theme
        .extends
        .as_deref()
        .and_then(builtin_by_id)
        .unwrap_or_else(|| matching_builtin(theme.theme_type));

    Ok(resolve_theme(&theme, Some(&base)))
}

pub fn theme_exists(paths: &AppPaths, theme_id: &str) -> bool {
    builtin_by_id(theme_id).is_some() || paths.themes_dir().join(format!("{theme_id}.json")).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-theme-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn 내장_다크_테마는_모든_시맨틱_토큰을_포함한다() {
        let theme = builtin_dark();
        for key in required_color_keys() {
            assert!(theme.colors.contains_key(&key), "missing dark color token: {key}");
        }
        for key in required_syntax_keys() {
            assert!(theme.syntax.contains_key(key), "missing dark syntax token: {key}");
        }
        for key in required_terminal_keys() {
            assert!(theme.terminal.contains_key(key), "missing dark terminal token: {key}");
        }
    }

    #[test]
    fn 내장_라이트_테마는_모든_시맨틱_토큰을_포함한다() {
        let theme = builtin_light();
        for key in required_color_keys() {
            assert!(theme.colors.contains_key(&key), "missing light color token: {key}");
        }
        for key in required_syntax_keys() {
            assert!(theme.syntax.contains_key(key), "missing light syntax token: {key}");
        }
        for key in required_terminal_keys() {
            assert!(theme.terminal.contains_key(key), "missing light terminal token: {key}");
        }
    }

    #[test]
    fn 터미널_ansi_16색이_전부_존재한다() {
        let ansi = [
            "black",
            "red",
            "green",
            "yellow",
            "blue",
            "magenta",
            "cyan",
            "white",
            "brightBlack",
            "brightRed",
            "brightGreen",
            "brightYellow",
            "brightBlue",
            "brightMagenta",
            "brightCyan",
            "brightWhite",
        ];
        for key in ansi {
            assert!(builtin_dark().terminal.contains_key(key));
            assert!(builtin_light().terminal.contains_key(key));
        }
    }

    #[test]
    fn palette_참조는_해석된다() {
        let mut palette = BTreeMap::new();
        palette.insert("accent".to_string(), "#112233".to_string());
        let mut colors = BTreeMap::new();
        colors.insert("app.accent".to_string(), "$accent".to_string());

        let theme = Theme {
            version: THEME_SCHEMA_VERSION,
            id: "custom".to_string(),
            name: "Custom".to_string(),
            theme_type: ThemeType::Dark,
            extends: None,
            palette,
            colors,
            syntax: BTreeMap::new(),
            terminal: BTreeMap::new(),
        };

        let resolved = resolve_theme(&theme, None);
        assert_eq!(resolved.colors.get("app.accent"), Some(&"#112233".to_string()));
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn 없는_팔레트_참조는_경고를_남기고_원문을_유지한다() {
        let mut colors = BTreeMap::new();
        colors.insert("app.accent".to_string(), "$missing".to_string());

        let theme = Theme {
            version: THEME_SCHEMA_VERSION,
            id: "custom".to_string(),
            name: "Custom".to_string(),
            theme_type: ThemeType::Dark,
            extends: None,
            palette: BTreeMap::new(),
            colors,
            syntax: BTreeMap::new(),
            terminal: BTreeMap::new(),
        };

        let resolved = resolve_theme(&theme, None);
        assert_eq!(resolved.colors.get("app.accent"), Some(&"$missing".to_string()));
        assert!(!resolved.warnings.is_empty());
    }

    #[test]
    fn extends로_부분_오버라이드시_base가_나머지를_채운다() {
        let mut colors = BTreeMap::new();
        colors.insert("app.accent".to_string(), "#abcdef".to_string());

        let child = Theme {
            version: THEME_SCHEMA_VERSION,
            id: "custom-dark".to_string(),
            name: "Custom Dark".to_string(),
            theme_type: ThemeType::Dark,
            extends: Some(BUILTIN_DARK_ID.to_string()),
            palette: BTreeMap::new(),
            colors,
            syntax: BTreeMap::new(),
            terminal: BTreeMap::new(),
        };

        let base = builtin_dark();
        let resolved = resolve_theme(&child, Some(&base));

        assert_eq!(resolved.colors.get("app.accent"), Some(&"#abcdef".to_string()));
        assert_eq!(resolved.colors.get("app.background"), base.colors.get("app.background"));
        assert!(!resolved.warnings.is_empty());
        for key in required_color_keys() {
            assert!(resolved.colors.contains_key(&key));
        }
    }

    #[test]
    fn list_themes는_내장_2종과_사용자_테마를_반환하고_파손파일은_제외한다() {
        let data_dir = temp_data_dir("list");
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");

        let mut user_theme = builtin_light();
        user_theme.id = "my-light".to_string();
        user_theme.name = "My Light".to_string();
        persist::write_json(&paths.themes_dir().join("my-light.json"), &user_theme).expect("write valid theme");
        std::fs::write(paths.themes_dir().join("broken.json"), b"{not json").expect("write broken theme");

        let list = list_themes(&paths);

        assert_eq!(list.len(), 3);
        assert!(list.iter().any(|summary| summary.id == BUILTIN_DARK_ID && summary.builtin));
        assert!(list.iter().any(|summary| summary.id == BUILTIN_LIGHT_ID && summary.builtin));
        assert!(list.iter().any(|summary| summary.id == "my-light" && !summary.builtin));

        std::fs::remove_dir_all(paths.themes_dir()).ok();
    }

    #[test]
    fn load_theme는_내장_아이디를_즉시_해석한다() {
        let paths = AppPaths::new(temp_data_dir("load-builtin"));
        let resolved = load_theme(&paths, BUILTIN_DARK_ID).expect("load builtin");
        assert_eq!(resolved.id, BUILTIN_DARK_ID);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn load_theme는_없는_아이디에_notfound를_반환한다() {
        let paths = AppPaths::new(temp_data_dir("load-missing"));
        let result = load_theme(&paths, "does-not-exist");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
