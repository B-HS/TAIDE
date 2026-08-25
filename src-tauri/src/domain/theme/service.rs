use std::collections::BTreeMap;

use crate::domain::theme::types::{ResolvedTheme, SyntaxStyle, Theme, ThemeSummary, ThemeType, TokenColorRule, THEME_SCHEMA_VERSION};
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::infra::root_guard;
use crate::paths::AppPaths;

pub const BUILTIN_DARK_ID: &str = "taide-dark";
pub const BUILTIN_LIGHT_ID: &str = "taide-light";

const BUNDLED_THEME_SOURCES: &[(&str, &str)] = &[
    ("one-dark-pro", include_str!("../../../resources/themes/one-dark-pro.json")),
    ("dracula", include_str!("../../../resources/themes/dracula.json")),
    ("github-dark", include_str!("../../../resources/themes/github-dark.json")),
    ("github-light", include_str!("../../../resources/themes/github-light.json")),
    ("tokyo-night", include_str!("../../../resources/themes/tokyo-night.json")),
    ("catppuccin-mocha", include_str!("../../../resources/themes/catppuccin-mocha.json")),
    ("nord", include_str!("../../../resources/themes/nord.json")),
    ("gruvbox-dark", include_str!("../../../resources/themes/gruvbox-dark.json")),
    ("monokai", include_str!("../../../resources/themes/monokai.json")),
    ("solarized-light", include_str!("../../../resources/themes/solarized-light.json")),
    ("vscode-abyss", include_str!("../../../resources/themes/vscode-abyss.json")),
    (
        "vscode-monokai-dimmed",
        include_str!("../../../resources/themes/vscode-monokai-dimmed.json"),
    ),
    (
        "vscode-solarized-dark",
        include_str!("../../../resources/themes/vscode-solarized-dark.json"),
    ),
    (
        "vscode-tomorrow-night-blue",
        include_str!("../../../resources/themes/vscode-tomorrow-night-blue.json"),
    ),
    (
        "intellij-islands-light",
        include_str!("../../../resources/themes/intellij-islands-light.json"),
    ),
    ("ayu-dark", include_str!("../../../resources/themes/ayu-dark.json")),
    ("ayu-light", include_str!("../../../resources/themes/ayu-light.json")),
    ("palenight", include_str!("../../../resources/themes/palenight.json")),
    ("night-owl", include_str!("../../../resources/themes/night-owl.json")),
    ("night-owl-light", include_str!("../../../resources/themes/night-owl-light.json")),
    ("rose-pine", include_str!("../../../resources/themes/rose-pine.json")),
    ("rose-pine-dawn", include_str!("../../../resources/themes/rose-pine-dawn.json")),
    ("everforest-dark", include_str!("../../../resources/themes/everforest-dark.json")),
    ("everforest-light", include_str!("../../../resources/themes/everforest-light.json")),
    ("kanagawa-wave", include_str!("../../../resources/themes/kanagawa-wave.json")),
    ("vitesse-dark", include_str!("../../../resources/themes/vitesse-dark.json")),
    ("vitesse-light", include_str!("../../../resources/themes/vitesse-light.json")),
    ("one-monokai", include_str!("../../../resources/themes/one-monokai.json")),
    ("vscode-dark-plus", include_str!("../../../resources/themes/vscode-dark-plus.json")),
    (
        "vscode-light-plus",
        include_str!("../../../resources/themes/vscode-light-plus.json"),
    ),
    (
        "vscode-dark-modern",
        include_str!("../../../resources/themes/vscode-dark-modern.json"),
    ),
    (
        "vscode-light-modern",
        include_str!("../../../resources/themes/vscode-light-modern.json"),
    ),
    (
        "vscode-kimbie-dark",
        include_str!("../../../resources/themes/vscode-kimbie-dark.json"),
    ),
    ("vscode-red", include_str!("../../../resources/themes/vscode-red.json")),
    (
        "vscode-quiet-light",
        include_str!("../../../resources/themes/vscode-quiet-light.json"),
    ),
    ("darcula", include_str!("../../../resources/themes/darcula.json")),
];

const COLOR_NAMESPACES: &[(&str, &[&str])] = &[
    ("app", &["background", "foreground", "border", "focusBorder", "shadow", "accent"]),
    (
        "appSidebar",
        &[
            "background",
            "itemHover",
            "itemActive",
            "iconDefault",
            "iconAgentRunning",
            "iconAgentWorking",
            "iconAgentAwaiting",
            "iconAgentIdle",
            "iconAgentUnknown",
            "badge",
        ],
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
        ("appSidebar.iconAgentWorking", "#89b4fa"),
        ("appSidebar.iconAgentAwaiting", "#f9e2af"),
        ("appSidebar.iconAgentIdle", "#a6e3a1"),
        ("appSidebar.iconAgentUnknown", "#6c7086"),
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
        ("explorer.itemSelected", "#585b70"),
        ("explorer.itemFocused", "#45475a"),
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
        ("appSidebar.iconAgentWorking", "#1e66f5"),
        ("appSidebar.iconAgentAwaiting", "#df8e1d"),
        ("appSidebar.iconAgentIdle", "#40a02b"),
        ("appSidebar.iconAgentUnknown", "#8c8fa1"),
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
        ("explorer.itemSelected", "#acb0be"),
        ("explorer.itemFocused", "#bcc0cc"),
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
        ("panel.matchHighlight", "#8839ef"),
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
        token_colors: None,
        author: None,
        license: None,
        source: None,
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
        token_colors: None,
        author: None,
        license: None,
        source: None,
    }
}

fn bundled_by_id(theme_id: &str) -> Option<Theme> {
    let (_, source) = BUNDLED_THEME_SOURCES.iter().find(|(id, _)| *id == theme_id)?;
    serde_json::from_str::<Theme>(source).ok()
}

pub fn bundled_themes() -> Vec<Theme> {
    BUNDLED_THEME_SOURCES
        .iter()
        .filter_map(|(id, source)| serde_json::from_str::<Theme>(source).ok().map(|theme| (id, theme)))
        .map(|(_, theme)| theme)
        .collect()
}

pub fn builtin_by_id(theme_id: &str) -> Option<Theme> {
    match theme_id {
        BUILTIN_DARK_ID => Some(builtin_dark()),
        BUILTIN_LIGHT_ID => Some(builtin_light()),
        _ => bundled_by_id(theme_id),
    }
}

pub fn builtin_id_for_system(system_theme: &str) -> &'static str {
    if system_theme.eq_ignore_ascii_case("light") {
        BUILTIN_LIGHT_ID
    } else {
        BUILTIN_DARK_ID
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

fn resolve_token_colors(theme: &Theme, base: Option<&Theme>) -> Option<Vec<TokenColorRule>> {
    theme
        .token_colors
        .clone()
        .or_else(|| base.and_then(|base_theme| base_theme.token_colors.clone()))
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
    let syntax_overrides: Vec<String> = match base {
        Some(_) => theme.syntax.keys().cloned().collect(),
        None => Vec::new(),
    };
    let token_colors = resolve_token_colors(theme, base);

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
        token_colors,
        syntax_overrides,
        warnings,
        author: theme.author.clone(),
        license: theme.license.clone(),
        source: theme.source.clone(),
    }
}

pub fn list_themes(paths: &AppPaths) -> Vec<ThemeSummary> {
    let mut list = vec![summarize(&builtin_dark(), true), summarize(&builtin_light(), true)];
    list.extend(bundled_themes().iter().map(|theme| summarize(theme, true)));

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

pub fn save_theme(paths: &AppPaths, theme: &Theme) -> AppResult<ThemeSummary> {
    if theme.id.trim().is_empty() {
        return Err(AppError::InvalidArgument("theme id must not be empty".to_string()));
    }
    if builtin_by_id(&theme.id).is_some() {
        return Err(AppError::InvalidArgument(format!("cannot overwrite builtin theme: {}", theme.id)));
    }
    if theme.id.contains(['/', '\\', '.']) {
        return Err(AppError::InvalidArgument(format!("invalid theme id: {}", theme.id)));
    }

    std::fs::create_dir_all(paths.themes_dir())?;
    persist::write_json(&paths.themes_dir().join(format!("{}.json", theme.id)), theme)?;
    Ok(summarize(theme, false))
}

/// `save_theme` above rejects a `theme.id` containing `/`, `\`, or `.` before it ever reaches a
/// path join, but `delete_theme`/`load_theme` used to build `themes_dir().join(theme_id)` straight
/// from their caller-supplied `theme_id` with no such check — a `theme_id` of `"../../../.ssh/id_rsa"`
/// (or any other `..`-laden value) escaped `themes_dir()` entirely, turning `delete_theme` into an
/// arbitrary-file-delete primitive reachable from the same remote surface `theme_delete`/`theme_get`
/// expose. Reuses `root_guard::ensure_safe_component` (the same single-path-segment guard
/// `file_mirror_untitled`/`file_clear_untitled_mirror` already apply to `tab_id`) rather than
/// duplicating `save_theme`'s ad hoc check. See
/// `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.4 (#7).
pub fn delete_theme(paths: &AppPaths, theme_id: &str) -> AppResult<()> {
    root_guard::ensure_safe_component(theme_id)?;
    if builtin_by_id(theme_id).is_some() {
        return Err(AppError::InvalidArgument(format!("cannot delete builtin theme: {theme_id}")));
    }
    let path = paths.themes_dir().join(format!("{theme_id}.json"));
    if !path.exists() {
        return Err(AppError::NotFound(format!("theme not found: {theme_id}")));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

pub fn load_theme(paths: &AppPaths, theme_id: &str) -> AppResult<ResolvedTheme> {
    if let Some(theme) = builtin_by_id(theme_id) {
        return Ok(resolve_theme(&theme, None));
    }

    root_guard::ensure_safe_component(theme_id)?;
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
    use std::collections::BTreeSet;

    use regex::Regex;

    use super::*;
    use crate::domain::theme::types::TokenColorSettings;
    use crate::error::AppErrorKind;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-theme-{name}-{}", uuid::Uuid::new_v4()))
    }

    /// The full theme catalog surfaced to users: every bundled JSON theme (`bundled_themes()`,
    /// 36 today) plus the two Rust-literal builtin themes (`builtin_dark()`/`builtin_light()`),
    /// which `bundled_themes()` alone omits. Production code (`list_themes`, `builtin_by_id`)
    /// already assembles builtin + bundled + user themes itself; this test-only helper exists so
    /// the data-quality lints below share one 38-theme iteration source instead of each hand-rolling
    /// its own `bundled_themes().chain(...)`. See
    /// `docs/acknowledge/2026-08-25-d36-theme-catalog-audit-contract.md` §1-b — before this helper,
    /// all five lints below iterated `bundled_themes()` only, so a defect exclusive to
    /// `builtin_dark`/`builtin_light` (as `taide-light`'s `panel.matchHighlight` was, per that
    /// contract's §0) could ship uncaught by any of them.
    fn theme_catalog() -> Vec<Theme> {
        bundled_themes().into_iter().chain([builtin_dark(), builtin_light()]).collect()
    }

    /// Normalizes a theme color string for defect-lint comparison: lowercases and expands it to an
    /// 8-digit `rrggbbaa` hex so equivalent shorthand and alpha-bearing forms compare equal —
    /// `#04395E`, `#04395eff`, and (were a bundled theme ever to use it) `#049e` all normalize to
    /// the same string. A bare 3-digit shorthand (`#abc`, no alpha channel) and a bare 6-digit hex
    /// are both treated as fully opaque and padded with `ff`; a 4-digit shorthand (`#abcf`) carries
    /// its own alpha nibble, which is duplicated like the color nibbles rather than overwritten, so
    /// a translucent 4-digit value never normalizes down to an opaque one. An explicit non-`ff`
    /// alpha (e.g. `#47526640`) is preserved rather than stripped, so a translucent overlay never
    /// normalizes down to the same string as its opaque RGB — alpha is a real part of what makes two
    /// list-row backgrounds visually distinguishable, not noise to discard before comparing
    /// (docs/acknowledge/2026-08-20-theme-list-colors-contract.md).
    fn normalize_hex_color(value: &str) -> String {
        let trimmed = value.trim().trim_start_matches('#').to_ascii_lowercase();
        let expand_shorthand = |shorthand: &str| -> String { shorthand.chars().flat_map(|nibble| [nibble, nibble]).collect() };
        match trimmed.len() {
            3 => format!("{}ff", expand_shorthand(&trimmed)),
            4 => expand_shorthand(&trimmed),
            6 => format!("{trimmed}ff"),
            _ => trimmed,
        }
    }

    #[test]
    fn normalize_hex_color는_3자리_축약_hex_를_불투명_8자리로_확장한다() {
        assert_eq!(normalize_hex_color("#fc0"), "ffcc00ff");
    }

    #[test]
    fn normalize_hex_color는_4자리_축약_hex_의_알파_니블을_보존해_확장한다() {
        assert_eq!(normalize_hex_color("#fc0f"), "ffcc00ff");
        assert_eq!(normalize_hex_color("#fc08"), "ffcc0088");
    }

    #[test]
    fn 내장_테마_아이디로는_저장할_수_없다() {
        let dir = std::env::temp_dir().join(format!("taide-theme-save-{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::new(dir);
        let mut theme = builtin_dark();
        theme.id = BUILTIN_DARK_ID.to_string();
        assert!(save_theme(&paths, &theme).is_err());
    }

    #[test]
    fn 경로_구분자가_섞인_아이디는_거부한다() {
        let dir = std::env::temp_dir().join(format!("taide-theme-path-{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::new(dir);
        let mut theme = builtin_dark();
        theme.id = "../evil".to_string();
        assert!(save_theme(&paths, &theme).is_err());
    }

    #[test]
    fn 사용자_테마는_저장하고_목록에_나타난다() {
        let dir = std::env::temp_dir().join(format!("taide-theme-ok-{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::new(dir);
        let mut theme = builtin_dark();
        theme.id = "my-theme".to_string();
        theme.name = "My Theme".to_string();
        theme.extends = Some(BUILTIN_DARK_ID.to_string());
        let summary = save_theme(&paths, &theme).expect("save");
        assert!(!summary.builtin);
        assert!(list_themes(&paths).iter().any(|item| item.id == "my-theme"));
        delete_theme(&paths, "my-theme").expect("delete");
        assert!(!list_themes(&paths).iter().any(|item| item.id == "my-theme"));
    }

    #[test]
    fn 시스템_테마_문자열을_내장_테마_아이디로_매핑한다() {
        assert_eq!(builtin_id_for_system("light"), BUILTIN_LIGHT_ID);
        assert_eq!(builtin_id_for_system("Light"), BUILTIN_LIGHT_ID);
        assert_eq!(builtin_id_for_system("dark"), BUILTIN_DARK_ID);
        assert_eq!(builtin_id_for_system(""), BUILTIN_DARK_ID);
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
            token_colors: None,
            author: None,
            license: None,
            source: None,
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
            token_colors: None,
            author: None,
            license: None,
            source: None,
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
            token_colors: None,
            author: None,
            license: None,
            source: None,
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

        assert_eq!(list.len(), 2 + BUNDLED_THEME_SOURCES.len() + 1);
        assert!(list.iter().any(|summary| summary.id == BUILTIN_DARK_ID && summary.builtin));
        assert!(list.iter().any(|summary| summary.id == BUILTIN_LIGHT_ID && summary.builtin));
        assert!(list.iter().any(|summary| summary.id == "my-light" && !summary.builtin));

        std::fs::remove_dir_all(paths.themes_dir()).ok();
    }

    #[test]
    fn 카탈로그_테마는_모두_파싱되고_이름이_비어있지_않다() {
        let bundled = bundled_themes();
        assert_eq!(bundled.len(), BUNDLED_THEME_SOURCES.len());
        for theme in theme_catalog() {
            assert!(!theme.name.trim().is_empty(), "catalog theme missing name: {}", theme.id);
            assert!(theme.extends.is_none(), "catalog theme must not use extends: {}", theme.id);
        }
    }

    #[test]
    fn 번들_테마는_모두_시맨틱_토큰_전량을_포함하고_경고가_없다() {
        for theme in bundled_themes() {
            for key in required_color_keys() {
                assert!(
                    theme.colors.contains_key(&key),
                    "missing color token '{key}' in bundled theme '{}'",
                    theme.id
                );
            }
            for key in required_syntax_keys() {
                assert!(
                    theme.syntax.contains_key(key),
                    "missing syntax token '{key}' in bundled theme '{}'",
                    theme.id
                );
            }
            for key in required_terminal_keys() {
                assert!(
                    theme.terminal.contains_key(key),
                    "missing terminal token '{key}' in bundled theme '{}'",
                    theme.id
                );
            }

            let resolved = resolve_theme(&theme, None);
            assert!(
                resolved.warnings.is_empty(),
                "bundled theme '{}' has resolve warnings: {:?}",
                theme.id,
                resolved.warnings
            );

            let token_colors = theme.token_colors.as_deref().unwrap_or_default();
            assert!(!token_colors.is_empty(), "bundled theme '{}' has no tokenColors", theme.id);
        }
    }

    #[test]
    fn 카탈로그_테마는_app_전경색과_배경색이_서로_다르다() {
        for theme in theme_catalog() {
            assert_ne!(
                theme.colors.get("app.foreground"),
                theme.colors.get("app.background"),
                "catalog theme '{}' has app.foreground identical to app.background",
                theme.id
            );
        }
    }

    /// Themes legitimately exempt from `카탈로그_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다`,
    /// with the reason each is a deliberate design choice rather than a reintroduction of
    /// `docs/acknowledge/2026-08-20-theme-list-colors-contract.md`'s defect. Empty today — every
    /// catalog theme (bundled JSON + the two builtin Rust literals, per
    /// `docs/acknowledge/2026-08-25-d36-theme-catalog-audit-contract.md` §1-b) satisfies the
    /// invariant on its own resolved colors after that contract's fix.
    const LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS: &[(&str, &str)] = &[];

    #[test]
    fn 카탈로그_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            if LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS.iter().any(|(id, _)| *id == theme.id) {
                continue;
            }

            let active_raw = theme.colors.get("list.activeBackground");
            let panel = theme.colors.get("panel.background").map(|value| normalize_hex_color(value));
            let hover = theme.colors.get("list.hoverBackground").map(|value| normalize_hex_color(value));
            let active = active_raw.map(|value| normalize_hex_color(value));

            if active == panel {
                violations.push(format!(
                    "'{}': list.activeBackground({active_raw:?}) == panel.background — the selected row would render invisible",
                    theme.id
                ));
            }
            if active == hover {
                violations.push(format!(
                    "'{}': list.activeBackground({active_raw:?}) == list.hoverBackground — the active selection can't be told apart from a mere hover",
                    theme.id
                ));
            }
        }

        assert!(
            violations.is_empty(),
            "list color defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn 카탈로그_테마는_panel_매치_하이라이트가_불투명하다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            let Some(match_highlight_raw) = theme.colors.get("panel.matchHighlight") else {
                continue;
            };
            let normalized = normalize_hex_color(match_highlight_raw);

            if !normalized.ends_with("ff") {
                violations.push(format!(
                    "'{}': panel.matchHighlight({match_highlight_raw:?}) is translucent — this token renders as foreground text (search/palette match emphasis), so a translucent value gets absorbed by whatever sits behind it instead of composing a legible color",
                    theme.id
                ));
            }
        }

        assert!(
            violations.is_empty(),
            "panel.matchHighlight defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    /// Low-cost second gate against the exact defect fixed by
    /// `docs/acknowledge/2026-08-24-d33-restructure-carryover-contract.md` §3-C (three bundled
    /// themes shipped `panel.matchHighlight` hex-identical to `app.foreground`, so search/palette
    /// match emphasis rendered as invisible body text). The TS pipeline (`mapping-tables.ts`'s
    /// `isDistinctFromBodyForeground`) already runs a full CIE76 `deltaE76` check with a 2.3
    /// just-noticeable-difference threshold on every *derived* candidate before a bundled JSON is
    /// ever written, so this Rust lint deliberately does not re-implement CIE76: across the current
    /// 38-theme catalog (36 bundled + 2 builtin, since
    /// `docs/acknowledge/2026-08-25-d36-theme-catalog-audit-contract.md` §1-b widened this lint's
    /// iteration source from `bundled_themes()` to `theme_catalog()`) the identical-color defect
    /// always manifests as exact hex equality (ΔE 0.0), and the next-lowest real distinctness value
    /// is 5.39 (`one-monokai`) — comfortably above the 2.3 threshold — so a hex-equality check and a
    /// ΔE<2.3 check agree on every catalog theme today. This test's job is narrower and cheaper than
    /// the TS gate's: catch a catalog theme (bundled JSON or a `builtin_dark`/`builtin_light` Rust
    /// literal) that re-enters the exact-duplicate state (e.g. hand-edited or hardcoded outside the
    /// TS pipeline), not to arbitrate borderline perceptual closeness — that precision work stays on
    /// the TS side.
    #[test]
    fn 카탈로그_테마는_panel_매치_하이라이트가_app_전경색과_동일하지_않다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            let Some(match_highlight_raw) = theme.colors.get("panel.matchHighlight") else {
                continue;
            };
            let Some(foreground_raw) = theme.colors.get("app.foreground") else {
                continue;
            };

            if normalize_hex_color(match_highlight_raw) == normalize_hex_color(foreground_raw) {
                violations.push(format!(
                    "'{}': panel.matchHighlight({match_highlight_raw:?}) == app.foreground({foreground_raw:?}) — search/palette match emphasis would render indistinguishable from ordinary body text",
                    theme.id
                ));
            }
        }

        assert!(
            violations.is_empty(),
            "panel.matchHighlight/app.foreground identical-color defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    /// Matches TS `MIN_CONTRAST_RATIO` (`src/shared/lib/theme-convert/contrast.ts`) exactly — same
    /// value (3.0), same WCAG basis: the 3:1 non-text contrast minimum (WCAG 2.x Success Criterion
    /// 1.4.11) for UI components and graphical objects, which `panel.matchHighlight` — a foreground
    /// emphasis token layered over `panel.background`, not paragraph body text — falls under.
    const MATCH_HIGHLIGHT_MIN_CONTRAST: f64 = 3.0;

    const RGB_CHANNEL_MAX: f64 = 255.0;
    const SRGB_LINEAR_THRESHOLD: f64 = 0.03928;
    const SRGB_LINEAR_DIVISOR: f64 = 12.92;
    const SRGB_GAMMA_OFFSET: f64 = 0.055;
    const SRGB_GAMMA_DIVISOR: f64 = 1.055;
    const SRGB_GAMMA_EXPONENT: f64 = 2.4;
    const LUMINANCE_WEIGHT_R: f64 = 0.2126;
    const LUMINANCE_WEIGHT_G: f64 = 0.7152;
    const LUMINANCE_WEIGHT_B: f64 = 0.0722;
    const CONTRAST_RATIO_OFFSET: f64 = 0.05;

    /// Parses the RGB channels out of a theme color string, built on `normalize_hex_color` above
    /// (which already handles 3/4/6/8-digit shorthand and lowercasing) and reading only its first
    /// 6 hex digits — an alpha suffix, if present, is ignored. This mirrors TS `hexToRgb`
    /// (`shared/lib/color.ts`) called directly (as `contrast.ts`'s local `relativeLuminance` does),
    /// not `compositeOverBackground`'s alpha-composited path: this lint deliberately does not port
    /// alpha compositing. `카탈로그_테마는_panel_매치_하이라이트가_불투명하다` above is a separate,
    /// independent `#[test]` — cargo gives no ordering guarantee between it and this gate, so its
    /// rejection of a translucent `panel.matchHighlight` cannot be relied on to run "before" this
    /// one; it catches translucent values on its own, not as a precondition this gate depends on.
    /// The omission is justified empirically instead: across the current 38-theme catalog (36
    /// bundled JSON files plus the two builtin Rust literals), every `panel.background` and
    /// `panel.matchHighlight` is a plain 6-digit opaque hex today, so alpha compositing would be an
    /// identity operation on every input this function actually receives.
    fn hex_to_rgb(value: &str) -> Option<(f64, f64, f64)> {
        let normalized = normalize_hex_color(value);
        if normalized.len() != 8 || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let r = u8::from_str_radix(&normalized[0..2], 16).ok()?;
        let g = u8::from_str_radix(&normalized[2..4], 16).ok()?;
        let b = u8::from_str_radix(&normalized[4..6], 16).ok()?;
        Some((f64::from(r), f64::from(g), f64::from(b)))
    }

    /// Ports TS `srgbChannelToLinear` (`contrast.ts`) — the sRGB electro-optical transfer function
    /// (IEC 61966-2-1) that WCAG's relative-luminance formula requires before applying the R/G/B
    /// weights.
    fn srgb_channel_to_linear(channel: f64) -> f64 {
        let normalized = channel / RGB_CHANNEL_MAX;
        if normalized <= SRGB_LINEAR_THRESHOLD {
            normalized / SRGB_LINEAR_DIVISOR
        } else {
            ((normalized + SRGB_GAMMA_OFFSET) / SRGB_GAMMA_DIVISOR).powf(SRGB_GAMMA_EXPONENT)
        }
    }

    /// Ports TS `relativeLuminance` (`contrast.ts`) — WCAG 2.x relative luminance of an sRGB color.
    fn relative_luminance(hex: &str) -> Option<f64> {
        let (r, g, b) = hex_to_rgb(hex)?;
        Some(
            LUMINANCE_WEIGHT_R * srgb_channel_to_linear(r)
                + LUMINANCE_WEIGHT_G * srgb_channel_to_linear(g)
                + LUMINANCE_WEIGHT_B * srgb_channel_to_linear(b),
        )
    }

    /// Ports TS `contrastRatio` (`contrast.ts`) — the WCAG 2.x contrast-ratio formula between two
    /// colors' relative luminances.
    fn contrast_ratio(hex_a: &str, hex_b: &str) -> Option<f64> {
        let luminance_a = relative_luminance(hex_a)?;
        let luminance_b = relative_luminance(hex_b)?;
        let lighter = luminance_a.max(luminance_b);
        let darker = luminance_a.min(luminance_b);
        Some((lighter + CONTRAST_RATIO_OFFSET) / (darker + CONTRAST_RATIO_OFFSET))
    }

    /// Catalog themes whose `panel.matchHighlight` cannot clear `MATCH_HIGHLIGHT_MIN_CONTRAST`
    /// against `panel.background` without abandoning the theme's own accent hue — mirrors TS
    /// `MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`
    /// (`src/shared/lib/theme-convert/bundled-theme-contrast.test.ts`) with the identical two
    /// entries and reasoning: in both cases the upstream source defines its accent (VS Code's
    /// `list.highlightForeground`) as exactly one shade, with no darker same-hue variant to
    /// substitute — unlike `github-dark`/`github-light`, whose upstream ships a full lightness scale
    /// for the accent color, letting a darker same-hue value be hand-picked from that scale instead,
    /// so no exemption was needed for those two (see `docs/theme-system.md` §8.2.3's table of 7
    /// hand-fixed `panel.matchHighlight` themes). Falling back to a generic `editor.foreground`
    /// candidate is deliberately not applied to bundled data either, since it would replace the
    /// accent with a neutral gray and erase the theme's identity. See
    /// `docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md` §3-A for the per-theme upstream
    /// palette investigation this pair of entries is based on.
    const MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS: &[(&str, &str)] = &[
        (
            "everforest-light",
            "upstream foreground palette (sainnhe/everforest-vscode src/palette/light/foreground.ts) has one shade per named accent — 'green' (#8da101, the source of list.highlightForeground) has no darker variant, only the lighter 'dimGreen' (#a4bb4a)",
        ),
        (
            "rose-pine-dawn",
            "upstream Rose Pine Dawn palette defines exactly one shade per named color — 'rose' (#d7827e, the source of list.highlightForeground) has no darker variant; the nearest hue, 'love' (#b4637a), is a distinct accent already used for errors, not a shade of rose",
        ),
    ];

    /// WCAG contrast gate (3) for `panel.matchHighlight` vs `panel.background`, ported from TS
    /// `validateOutputColors`'s `matchHighlight` pair (`contrast.ts`'s `CONTRAST_PAIRS`) — see
    /// `docs/acknowledge/2026-08-25-d36-theme-catalog-audit-contract.md` §1-c. Runs over the full
    /// 38-theme catalog (36 bundled + 2 builtin, via `theme_catalog()`), closing the structural gap
    /// the opacity-only `카탈로그_테마는_panel_매치_하이라이트가_불투명하다` above leaves: an opaque
    /// 6-digit hex can still be arbitrarily low-contrast, which is exactly how `taide-light`'s
    /// `panel.matchHighlight` (`#df8e1d`, 2.15 against `panel.background` `#e6e9ef`) shipped
    /// unnoticed — `bundled_themes()`-only lints never covered the two builtin Rust literals, and
    /// the opacity lint only checks the alpha channel, not the ratio itself.
    #[test]
    fn 카탈로그_테마는_panel_매치_하이라이트가_패널_배경과_최소_대비를_가진다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            if MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS.iter().any(|(id, _)| *id == theme.id) {
                continue;
            }
            let Some(match_highlight_raw) = theme.colors.get("panel.matchHighlight") else {
                continue;
            };
            let Some(panel_background_raw) = theme.colors.get("panel.background") else {
                continue;
            };

            match contrast_ratio(match_highlight_raw, panel_background_raw) {
                Some(ratio) if ratio >= MATCH_HIGHLIGHT_MIN_CONTRAST => {}
                Some(ratio) => violations.push(format!(
                    "'{}': panel.matchHighlight({match_highlight_raw:?}) vs panel.background({panel_background_raw:?}) = {ratio:.2} (최소 {MATCH_HIGHLIGHT_MIN_CONTRAST})",
                    theme.id
                )),
                None => violations.push(format!(
                    "'{}': panel.matchHighlight({match_highlight_raw:?}) 또는 panel.background({panel_background_raw:?}) hex 파싱 실패",
                    theme.id
                )),
            }
        }

        assert!(
            violations.is_empty(),
            "panel.matchHighlight contrast defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn 매치_하이라이트_대비_예외_등재분은_실제로_최소_대비에_미달한다() {
        let catalog = theme_catalog();

        for (exempt_id, _reason) in MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS {
            let theme = catalog
                .iter()
                .find(|theme| theme.id == *exempt_id)
                .unwrap_or_else(|| panic!("exempted theme '{exempt_id}' not found in catalog"));
            let match_highlight_raw = theme
                .colors
                .get("panel.matchHighlight")
                .unwrap_or_else(|| panic!("'{exempt_id}' has no panel.matchHighlight"));
            let panel_background_raw = theme
                .colors
                .get("panel.background")
                .unwrap_or_else(|| panic!("'{exempt_id}' has no panel.background"));
            let ratio = contrast_ratio(match_highlight_raw, panel_background_raw)
                .unwrap_or_else(|| panic!("'{exempt_id}' panel.matchHighlight/panel.background hex 파싱 실패"));

            assert!(
                ratio < MATCH_HIGHLIGHT_MIN_CONTRAST,
                "'{exempt_id}' is listed in MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS but its contrast {ratio:.2} already meets MATCH_HIGHLIGHT_MIN_CONTRAST — remove the exemption",
            );
        }
    }

    #[test]
    fn 번들_테마_아이디로는_저장하거나_삭제할_수_없다() {
        let paths = AppPaths::new(temp_data_dir("bundled-guard"));
        let mut theme = builtin_dark();
        theme.id = "dracula".to_string();
        assert!(save_theme(&paths, &theme).is_err());
        assert!(delete_theme(&paths, "dracula").is_err());
    }

    #[test]
    fn delete_theme는_경로_구분자가_섞인_아이디로_저장소_밖_파일을_지울_수_없다() {
        let dir = temp_data_dir("delete-traversal");
        let paths = AppPaths::new(dir.clone());
        std::fs::create_dir_all(paths.themes_dir()).unwrap();
        let outside_file = dir.join("secret.json");
        std::fs::write(&outside_file, "{}").unwrap();

        let result = delete_theme(&paths, "../secret");

        assert_eq!(result.unwrap_err().kind(), AppErrorKind::InvalidArgument);
        assert!(outside_file.exists(), "저장소 밖 파일은 지워지면 안 된다");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_theme는_경로_구분자가_섞인_아이디를_거부한다() {
        let paths = AppPaths::new(temp_data_dir("load-traversal"));
        let result = load_theme(&paths, "../../etc/passwd");
        assert_eq!(result.unwrap_err().kind(), AppErrorKind::InvalidArgument);
    }

    #[test]
    fn 번들_테마는_extends의_base로_해석된다() {
        let mut colors = BTreeMap::new();
        colors.insert("app.accent".to_string(), "#abcdef".to_string());

        let child = Theme {
            version: THEME_SCHEMA_VERSION,
            id: "custom-from-dracula".to_string(),
            name: "Custom From Dracula".to_string(),
            theme_type: ThemeType::Dark,
            extends: Some("dracula".to_string()),
            palette: BTreeMap::new(),
            colors,
            syntax: BTreeMap::new(),
            terminal: BTreeMap::new(),
            token_colors: None,
            author: None,
            license: None,
            source: None,
        };

        let base = builtin_by_id("dracula").expect("dracula bundled theme resolves");
        let resolved = resolve_theme(&child, Some(&base));

        assert_eq!(resolved.colors.get("app.accent"), Some(&"#abcdef".to_string()));
        for key in required_color_keys() {
            assert!(resolved.colors.contains_key(&key));
        }
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

    fn keyword_rule(fg: &str) -> TokenColorRule {
        TokenColorRule {
            scope: vec!["keyword".to_string()],
            settings: TokenColorSettings {
                foreground: Some(fg.to_string()),
                background: None,
                font_style: None,
            },
        }
    }

    #[test]
    fn 자식의_token_colors가_none이면_base에서_상속된다() {
        let mut base = builtin_dark();
        base.token_colors = Some(vec![keyword_rule("#cba6f7")]);

        let mut child = builtin_dark();
        child.id = "custom-dark".to_string();
        child.extends = Some(base.id.clone());
        child.token_colors = None;

        let resolved = resolve_theme(&child, Some(&base));
        assert_eq!(resolved.token_colors, base.token_colors);
    }

    #[test]
    fn 자식의_token_colors가_some이면_base를_완전히_교체한다() {
        let mut base = builtin_dark();
        base.token_colors = Some(vec![keyword_rule("#cba6f7")]);

        let mut child = builtin_dark();
        child.id = "custom-dark".to_string();
        child.extends = Some(base.id.clone());
        child.token_colors = Some(vec![keyword_rule("#ff0000")]);

        let resolved = resolve_theme(&child, Some(&base));
        assert_eq!(resolved.token_colors, child.token_colors);
        assert_ne!(resolved.token_colors, base.token_colors);
    }

    #[test]
    fn syntax_overrides는_자식이_명시한_syntax_키만_담는다() {
        let base = builtin_dark();

        let mut child = builtin_dark();
        child.id = "custom-dark".to_string();
        child.extends = Some(base.id.clone());
        child.syntax = syntax_from_pairs(&[("keyword", "#ff0000", false, false), ("string", "#00ff00", false, false)]);

        let resolved = resolve_theme(&child, Some(&base));
        assert_eq!(resolved.syntax_overrides, vec!["keyword".to_string(), "string".to_string()]);
    }

    #[test]
    fn base가_없는_루트_테마는_syntax_overrides가_비어있다() {
        let resolved = resolve_theme(&builtin_dark(), None);
        assert!(resolved.syntax_overrides.is_empty());
    }

    #[test]
    fn 번들_테마_전체는_base가_없어_syntax_overrides가_비어있다() {
        for theme in bundled_themes() {
            let resolved = resolve_theme(&theme, None);
            assert!(
                resolved.syntax_overrides.is_empty(),
                "{} must have empty syntax_overrides",
                theme.id
            );
        }
    }

    /// Extracts a `[...] as const` array literal's quoted string members, starting at `start_marker`
    /// (typically an `export const NAME = [` declaration) — the flat-list half of the two token-shape
    /// extractions [`ts_color_token_keys`] needs.
    ///
    /// The member pattern (`'([a-zA-Z0-9]+)'`) only matches single-segment alphanumeric token names —
    /// a token renamed to include `.`/`-`/`_` (e.g. `variable.parameter`) would either not match at
    /// all or match only part of itself, silently dropping (or truncating) it from the returned set
    /// rather than erroring. Every current token name in `theme-tokens.ts` is a single alphanumeric
    /// segment, so this holds today; [`ts_color_token_keys`]'s `token_pattern` below shares the exact
    /// same constraint. See `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md` §1 T1-E.
    fn extract_flat_string_list(source: &str, start_marker: &str) -> BTreeSet<String> {
        let start = source
            .find(start_marker)
            .unwrap_or_else(|| panic!("{start_marker} 를 찾을 수 없습니다"))
            + start_marker.len();
        let end = source[start..]
            .find("] as const")
            .unwrap_or_else(|| panic!("{start_marker} 의 끝(] as const)을 찾을 수 없습니다"));
        let block = &source[start..start + end];
        Regex::new(r"'([a-zA-Z0-9]+)'")
            .expect("유효한 정규식")
            .captures_iter(block)
            .map(|capture| capture[1].to_string())
            .collect()
    }

    /// Extracts `theme-tokens.ts`'s `COLOR_NAMESPACES` into the same flat `"namespace.token"` key
    /// shape [`required_color_keys`] returns on the Rust side.
    fn ts_color_token_keys(source: &str) -> BTreeSet<String> {
        let start = source
            .find("export const COLOR_NAMESPACES = [")
            .expect("COLOR_NAMESPACES 시작을 찾을 수 없습니다")
            + "export const COLOR_NAMESPACES = [".len();
        let end = source[start..]
            .find("] as const")
            .expect("COLOR_NAMESPACES 끝(] as const)을 찾을 수 없습니다");
        let block = &source[start..start + end];

        let namespace_pattern = Regex::new(r"id:\s*'([a-zA-Z0-9]+)',\s*tokens:\s*\[([^\]]*)\]").expect("유효한 정규식");
        let token_pattern = Regex::new(r"'([a-zA-Z0-9]+)'").expect("유효한 정규식");
        let mut keys = BTreeSet::new();
        for namespace_capture in namespace_pattern.captures_iter(block) {
            let namespace = &namespace_capture[1];
            for token_capture in token_pattern.captures_iter(&namespace_capture[2]) {
                keys.insert(format!("{namespace}.{}", &token_capture[1]));
            }
        }
        keys
    }

    /// `R5#9` — the ~200-token semantic key list (`COLOR_NAMESPACES`/`SYNTAX_TOKENS`/
    /// `TERMINAL_TOKENS`) is hand-mirrored between this Rust module and
    /// `src/entities/theme/theme-tokens.ts` with no generator and, until this test, no parity check —
    /// a token added to one side silently stops being either enforced (`내장_다크_테마는_모든_시맨틱_토큰을_포함한다`
    /// above) or exposed in the theme editor.
    #[test]
    fn 테마_토큰_목록은_rust와_theme_tokens_ts에서_일치한다() {
        let ts_source = include_str!("../../../../src/entities/theme/theme-tokens.ts");

        let rust_color_keys: BTreeSet<String> = required_color_keys().into_iter().collect();
        let ts_color_keys = ts_color_token_keys(ts_source);
        assert_eq!(
            rust_color_keys, ts_color_keys,
            "색상 토큰 목록이 Rust COLOR_NAMESPACES 와 TS COLOR_NAMESPACES 사이에서 다릅니다"
        );

        let rust_syntax_keys: BTreeSet<String> = required_syntax_keys().into_iter().map(str::to_string).collect();
        let ts_syntax_keys = extract_flat_string_list(ts_source, "export const SYNTAX_TOKENS = [");
        assert_eq!(
            rust_syntax_keys, ts_syntax_keys,
            "구문 강조 토큰 목록이 Rust SYNTAX_TOKENS 와 TS SYNTAX_TOKENS 사이에서 다릅니다"
        );

        let rust_terminal_keys: BTreeSet<String> = required_terminal_keys().into_iter().map(str::to_string).collect();
        let ts_terminal_keys = extract_flat_string_list(ts_source, "export const TERMINAL_TOKENS = [");
        assert_eq!(
            rust_terminal_keys, ts_terminal_keys,
            "터미널 ANSI 토큰 목록이 Rust TERMINAL_ANSI_TOKENS 와 TS TERMINAL_TOKENS 사이에서 다릅니다"
        );
    }
}
