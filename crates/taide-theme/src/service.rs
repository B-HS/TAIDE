use std::collections::BTreeMap;
use std::sync::OnceLock;

use taide_infra::persist;
use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::paths::AppPaths;
use taide_model::theme::{ResolvedTheme, SyntaxStyle, Theme, ThemeSummary, ThemeType, TokenColorRule, THEME_SCHEMA_VERSION};

pub const BUILTIN_DARK_ID: &str = "taide-dark";
pub const BUILTIN_LIGHT_ID: &str = "taide-light";

const BUNDLED_THEME_SOURCES: &[(&str, &str)] = &[
    ("one-dark-pro", include_str!("../resources/themes/one-dark-pro.json")),
    ("dracula", include_str!("../resources/themes/dracula.json")),
    ("github-dark", include_str!("../resources/themes/github-dark.json")),
    ("github-light", include_str!("../resources/themes/github-light.json")),
    ("tokyo-night", include_str!("../resources/themes/tokyo-night.json")),
    ("catppuccin-mocha", include_str!("../resources/themes/catppuccin-mocha.json")),
    ("nord", include_str!("../resources/themes/nord.json")),
    ("gruvbox-dark", include_str!("../resources/themes/gruvbox-dark.json")),
    ("monokai", include_str!("../resources/themes/monokai.json")),
    ("solarized-light", include_str!("../resources/themes/solarized-light.json")),
    ("vscode-abyss", include_str!("../resources/themes/vscode-abyss.json")),
    (
        "vscode-monokai-dimmed",
        include_str!("../resources/themes/vscode-monokai-dimmed.json"),
    ),
    (
        "vscode-solarized-dark",
        include_str!("../resources/themes/vscode-solarized-dark.json"),
    ),
    (
        "vscode-tomorrow-night-blue",
        include_str!("../resources/themes/vscode-tomorrow-night-blue.json"),
    ),
    (
        "intellij-islands-light",
        include_str!("../resources/themes/intellij-islands-light.json"),
    ),
    ("ayu-dark", include_str!("../resources/themes/ayu-dark.json")),
    ("ayu-light", include_str!("../resources/themes/ayu-light.json")),
    ("palenight", include_str!("../resources/themes/palenight.json")),
    ("night-owl", include_str!("../resources/themes/night-owl.json")),
    ("night-owl-light", include_str!("../resources/themes/night-owl-light.json")),
    ("rose-pine", include_str!("../resources/themes/rose-pine.json")),
    ("rose-pine-dawn", include_str!("../resources/themes/rose-pine-dawn.json")),
    ("everforest-dark", include_str!("../resources/themes/everforest-dark.json")),
    ("everforest-light", include_str!("../resources/themes/everforest-light.json")),
    ("kanagawa-wave", include_str!("../resources/themes/kanagawa-wave.json")),
    ("vitesse-dark", include_str!("../resources/themes/vitesse-dark.json")),
    ("vitesse-light", include_str!("../resources/themes/vitesse-light.json")),
    ("one-monokai", include_str!("../resources/themes/one-monokai.json")),
    ("vscode-dark-plus", include_str!("../resources/themes/vscode-dark-plus.json")),
    ("vscode-light-plus", include_str!("../resources/themes/vscode-light-plus.json")),
    ("vscode-dark-modern", include_str!("../resources/themes/vscode-dark-modern.json")),
    ("vscode-light-modern", include_str!("../resources/themes/vscode-light-modern.json")),
    ("vscode-kimbie-dark", include_str!("../resources/themes/vscode-kimbie-dark.json")),
    ("vscode-red", include_str!("../resources/themes/vscode-red.json")),
    ("vscode-quiet-light", include_str!("../resources/themes/vscode-quiet-light.json")),
    ("darcula", include_str!("../resources/themes/darcula.json")),
    (
        "visual-studio-cpp-dark",
        include_str!("../resources/themes/visual-studio-cpp-dark.json"),
    ),
    (
        "visual-studio-cpp-light",
        include_str!("../resources/themes/visual-studio-cpp-light.json"),
    ),
    ("catppuccin-latte", include_str!("../resources/themes/catppuccin-latte.json")),
    ("catppuccin-frappe", include_str!("../resources/themes/catppuccin-frappe.json")),
    (
        "catppuccin-macchiato",
        include_str!("../resources/themes/catppuccin-macchiato.json"),
    ),
    ("tokyo-night-storm", include_str!("../resources/themes/tokyo-night-storm.json")),
    ("tokyo-night-light", include_str!("../resources/themes/tokyo-night-light.json")),
    ("gruvbox-light", include_str!("../resources/themes/gruvbox-light.json")),
    ("rose-pine-moon", include_str!("../resources/themes/rose-pine-moon.json")),
    ("ayu-mirage", include_str!("../resources/themes/ayu-mirage.json")),
    ("github-dark-dimmed", include_str!("../resources/themes/github-dark-dimmed.json")),
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
        ("app.shadow", "#00000026"),
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
        ("tabBar.tabInactiveForeground", "#828596"),
        ("tabBar.tabBorder", "#ccd0da"),
        ("tabBar.tabActiveIndicator", "#1e66f5"),
        ("tabBar.dirtyDot", "#df8e1d"),
        ("tabBar.previewForeground", "#878a9b"),
        ("tabBar.dropTarget", "#1e66f5"),
        ("explorer.background", "#e6e9ef"),
        ("explorer.itemHover", "#ccd0da"),
        ("explorer.itemSelected", "#acb0be"),
        ("explorer.itemFocused", "#bcc0cc"),
        ("explorer.indentGuide", "#ccd0da"),
        ("explorer.folderIcon", "#1e66f5"),
        ("explorer.gitModified", "#ba7718"),
        ("explorer.gitAdded", "#3d9829"),
        ("explorer.gitDeleted", "#d20f39"),
        ("explorer.gitUntracked", "#179299"),
        ("explorer.gitIgnored", "#9ca0b0"),
        ("panel.background", "#e6e9ef"),
        ("panel.sectionHeader", "#6c6f85"),
        ("panel.inputBackground", "#eff1f5"),
        ("panel.inputBorder", "#ccd0da"),
        ("panel.matchHighlight", "#6611d4"),
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
        ("git.added", "#3d9829"),
        ("git.modified", "#ba7718"),
        ("git.deleted", "#d20f39"),
        ("git.renamed", "#1d92a6"),
        ("git.untracked", "#179299"),
        ("git.conflicted", "#e3590a"),
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
        ("statusIndicator.warning", "#ba7718"),
        ("statusIndicator.error", "#d20f39"),
        ("statusIndicator.success", "#3d9829"),
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
        ("input.placeholder", "#878b99"),
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

/// Counts how many times [`builtin_summaries`] actually parsed the bundled sources, so
/// `번들_요약은_list_themes_를_반복_호출해도_한_번만_파싱된다` can assert the re-parse count is zero
/// however many times `list_themes` runs. A work counter, not a wall-clock budget (계약 §C.2-3);
/// unlike a delta measured around `bundled_themes`, it stays correct while other tests run in
/// parallel in the same process.
#[cfg(test)]
static SUMMARY_PARSE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
fn summary_parse_count() -> u64 {
    SUMMARY_PARSE_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Summaries of the two Rust builtins plus every bundled JSON theme, parsed once per process.
///
/// [`list_themes`] used to rebuild this on every call, deserializing all of
/// [`BUNDLED_THEME_SOURCES`] (~1.1MB of JSON) into full [`Theme`] values just to read three fields
/// off each. `THEME.ALL` is invalidated by both `SettingsChanged` and `ThemeChanged`, so with the
/// settings window open every toggle paid that parse (research 3b §2-E).
///
/// Caching is sound here precisely because these sources are `include_str!` constants compiled into
/// the binary — nothing can change them while the process runs. User themes are the opposite: they
/// live under `paths.themes_dir()` and are read from disk on every [`list_themes`] call, so adding,
/// editing or deleting one is still reflected immediately.
fn builtin_summaries() -> &'static [ThemeSummary] {
    static SUMMARIES: OnceLock<Vec<ThemeSummary>> = OnceLock::new();
    SUMMARIES.get_or_init(|| {
        #[cfg(test)]
        SUMMARY_PARSE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut list = vec![summarize(&builtin_dark(), true), summarize(&builtin_light(), true)];
        list.extend(bundled_themes().iter().map(|theme| summarize(theme, true)));
        list
    })
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
    let mut list = builtin_summaries().to_vec();

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
    use taide_model::error::AppErrorKind;
    use taide_model::theme::TokenColorSettings;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-theme-{name}-{}", uuid::Uuid::new_v4()))
    }

    /// The full theme catalog surfaced to users: every bundled JSON theme (`bundled_themes()`,
    /// 47 today) plus the two Rust-literal builtin themes (`builtin_dark()`/`builtin_light()`),
    /// which `bundled_themes()` alone omits. Production code (`list_themes`, `builtin_by_id`)
    /// already assembles builtin + bundled + user themes itself; this test-only helper exists so
    /// the data-quality lints below share one 49-theme iteration source instead of each hand-rolling
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

    /// Enough repeats that a per-call parse would show up as a count far above 1. The assertion is
    /// on the parse count, never on elapsed time.
    const LIST_THEMES_REPEAT: usize = 20;

    #[test]
    fn 번들_요약은_list_themes_를_반복_호출해도_한_번만_파싱된다() {
        let paths = AppPaths::new(temp_data_dir("summary-cache"));

        for _ in 0..LIST_THEMES_REPEAT {
            let list = list_themes(&paths);
            assert_eq!(list.len(), 2 + BUNDLED_THEME_SOURCES.len());
        }

        assert_eq!(summary_parse_count(), 1, "번들 테마 요약은 프로세스당 한 번만 파싱돼야 한다");
    }

    #[test]
    fn 사용자_테마_추가와_삭제는_요약_캐시와_무관하게_즉시_반영된다() {
        let paths = AppPaths::new(temp_data_dir("summary-user"));
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");

        let builtin_only = list_themes(&paths).len();

        let mut user_theme = builtin_light();
        user_theme.id = "cache-probe".to_string();
        user_theme.name = "Cache Probe".to_string();
        let user_file = paths.themes_dir().join("cache-probe.json");
        persist::write_json(&user_file, &user_theme).expect("write user theme");

        let after_add = list_themes(&paths);
        assert_eq!(after_add.len(), builtin_only + 1);
        assert!(after_add.iter().any(|summary| summary.id == "cache-probe" && !summary.builtin));

        std::fs::remove_file(&user_file).expect("remove user theme");

        let after_remove = list_themes(&paths);
        assert_eq!(after_remove.len(), builtin_only);
        assert!(!after_remove.iter().any(|summary| summary.id == "cache-probe"));

        std::fs::remove_dir_all(paths.themes_dir()).ok();
    }

    #[test]
    fn 요약_캐시는_번들_테마_전량과_같은_내용을_준다() {
        let cached = builtin_summaries();
        let rebuilt: Vec<ThemeSummary> = std::iter::once(summarize(&builtin_dark(), true))
            .chain(std::iter::once(summarize(&builtin_light(), true)))
            .chain(bundled_themes().iter().map(|theme| summarize(theme, true)))
            .collect();

        assert_eq!(cached, rebuilt.as_slice());
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
    /// 49-theme catalog (47 bundled + 2 builtin, since
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
    /// 1.4.11) for UI components and graphical objects. Originally named `MATCH_HIGHLIGHT_MIN_CONTRAST`
    /// when it guarded a single pair (`panel.matchHighlight` vs `panel.background`,
    /// `docs/acknowledge/2026-08-25-d36-theme-catalog-audit-contract.md` §1-c); renamed here to match
    /// TS's already-generic name because `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md`
    /// §1-c reuses this same threshold for two more pairs below (`panel.matchHighlight`/`list.foreground`
    /// each vs `list.activeBackground`) — all three guard a foreground token layered over a UI surface,
    /// not paragraph body text.
    const MIN_CONTRAST_RATIO: f64 = 3.0;

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
    /// 6 hex digits — an alpha suffix, if present, is ignored, for both the foreground and the
    /// background argument every caller below passes. This mirrors TS `hexToRgb` (`shared/lib/color.ts`)
    /// called directly on the background half of a pair (as `contrast.ts`'s `foregroundContrastRatio`
    /// does via `relativeLuminance`), not `compositeOverBackground`'s alpha-composited path — TS itself
    /// never composites a *background* argument, only a translucent *foreground* one, so raw-RGB
    /// background reads have always been the shared, intentional behavior on both sides, not a Rust
    /// shortcut (`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §3-C's
    /// `rose-pine-dawn` exemption documents this same raw-RGB reading of an alpha-carrying background
    /// on the TS side, for the same `hexToRgb` reason).
    ///
    /// Not compositing the *foreground* argument (`panel.matchHighlight` or `list.foreground`, per
    /// caller) is justified empirically, same as before: across the current 49-theme catalog (47
    /// bundled JSON files plus the two builtin Rust literals), every `panel.matchHighlight` and
    /// `list.foreground` value is a plain 6-digit opaque hex today (verified by direct scan when
    /// `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-c added the two
    /// `list.activeBackground` pairs below), so `compositeOverBackground` would be an identity
    /// operation on every foreground this function actually receives — unlike `list.activeBackground`
    /// itself, which *does* carry real alpha in five catalog themes (`everforest-dark`,
    /// `everforest-light`, `night-owl`, `rose-pine-dawn`, `rose-pine`) and is read at raw RGB by
    /// design, per the paragraph above. `카탈로그_테마는_panel_매치_하이라이트가_불투명하다` above is a
    /// separate, independent `#[test]` guarding the same foreground-opacity fact for one of these two
    /// keys — cargo gives no ordering guarantee between it and the gates below, so its rejection of a
    /// translucent `panel.matchHighlight` cannot be relied on to run "before" them; it catches
    /// translucent values on its own, not as a precondition this function depends on.
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

    /// Catalog themes whose `panel.matchHighlight` cannot clear `MIN_CONTRAST_RATIO`
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
    /// 49-theme catalog (47 bundled + 2 builtin, via `theme_catalog()`), closing the structural gap
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
                Some(ratio) if ratio >= MIN_CONTRAST_RATIO => {}
                Some(ratio) => violations.push(format!(
                    "'{}': panel.matchHighlight({match_highlight_raw:?}) vs panel.background({panel_background_raw:?}) = {ratio:.2} (최소 {MIN_CONTRAST_RATIO})",
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
                ratio < MIN_CONTRAST_RATIO,
                "'{exempt_id}' is listed in MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS but its contrast {ratio:.2} already meets MIN_CONTRAST_RATIO — remove the exemption",
            );
        }
    }

    /// Selection-row axes (`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md`
    /// §0/§1-c): `panel.matchHighlight`/`list.foreground` against `list.activeBackground`, the row
    /// surface a *selected* palette/list entry actually paints (`docs/theme-system.md` §8.2, d-36 §4's
    /// render-path confirmation) — distinct from the `panel.background` axis the two lints above guard,
    /// which only governs the *unselected* row. Mirrors TS `SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`
    /// (`src/shared/lib/theme-convert/bundled-theme-contrast.test.ts`) — reason strings below are copied
    /// verbatim from there. Both entries already carry a `panel.matchHighlight` exemption above for the
    /// pre-existing `panel.background` axis; the reasons here are the same underlying upstream palette
    /// shortfall, re-verified against the different background.
    ///
    /// `nord` is deliberately not listed here (post-d40 review): the exemption's original premise held
    /// `list.activeBackground` fixed at the theme's bright 'frost' accent (`#88c0d0`) — but that same
    /// accent is also `panel.matchHighlight`/`app.accent`/`explorer.itemSelected`, so it was never
    /// actually immovable. Moving `list.activeBackground` to nord3 (`#4c566a`, the theme's own
    /// `list.inactiveSelectionBackground`) clears both selection-row axes at once without an exemption
    /// — see `crates/taide-theme/resources/themes/nord.json`.
    const SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS: &[(&str, &str)] = &[
        (
            "everforest-light",
            "same root cause as this theme's pre-existing panel.background exemption above — upstream sainnhe/everforest-vscode's foreground palette has exactly one shade for 'green' (#8da101, the source of list.highlightForeground/panel.matchHighlight), no darker variant to substitute. list.activeBackground (#e6e2cc, a ~50%-alpha overlay the gate reads at its raw opaque RGB) is darker than panel.background (raw #e6e2cc, L 0.756 vs #fdf6e3, L 0.923), so the same single-shade green loses contrast rather than gaining it.",
        ),
        (
            "rose-pine-dawn",
            "upstream rose-pine/vscode defines list.activeSelectionBackground as a near-transparent overlay (#6e6a8614, ~8% alpha) — TAIDE's contrast gate measures an alpha-carrying background at its raw RGB (#6e6a86), which reads far darker than the overlay's actual on-screen appearance over the light base. Combined with this theme's pre-existing panel.background shortfall (rose, #d7827e, has no darker upstream variant — see the exemption above), no candidate clears 3:1 against the gate's raw-RGB reading of list.activeBackground either.",
        ),
    ];

    /// WCAG contrast gate (3) for `panel.matchHighlight` vs `list.activeBackground` — the selected-row
    /// counterpart to `카탈로그_테마는_panel_매치_하이라이트가_패널_배경과_최소_대비를_가진다` above, closing the
    /// gap `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §0 identified: a theme
    /// can pass the `panel.background` axis (the unselected row) while still rendering illegible search/
    /// palette match emphasis the moment that same row is selected (`nord`'s `1.00` being the extreme
    /// case — see the exemption above). Runs over the full 49-theme catalog via `theme_catalog()`, same
    /// as every other lint in this module.
    #[test]
    fn 카탈로그_테마는_panel_매치_하이라이트가_선택_행_배경과_최소_대비를_가진다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            if SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS.iter().any(|(id, _)| *id == theme.id) {
                continue;
            }
            let Some(match_highlight_raw) = theme.colors.get("panel.matchHighlight") else {
                continue;
            };
            let Some(active_background_raw) = theme.colors.get("list.activeBackground") else {
                continue;
            };

            match contrast_ratio(match_highlight_raw, active_background_raw) {
                Some(ratio) if ratio >= MIN_CONTRAST_RATIO => {}
                Some(ratio) => violations.push(format!(
                    "'{}': panel.matchHighlight({match_highlight_raw:?}) vs list.activeBackground({active_background_raw:?}) = {ratio:.2} (최소 {MIN_CONTRAST_RATIO})",
                    theme.id
                )),
                None => violations.push(format!(
                    "'{}': panel.matchHighlight({match_highlight_raw:?}) 또는 list.activeBackground({active_background_raw:?}) hex 파싱 실패",
                    theme.id
                )),
            }
        }

        assert!(
            violations.is_empty(),
            "selection-row panel.matchHighlight contrast defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn 선택_행_매치_하이라이트_대비_예외_등재분은_실제로_최소_대비에_미달한다() {
        let catalog = theme_catalog();

        for (exempt_id, _reason) in SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS {
            let theme = catalog
                .iter()
                .find(|theme| theme.id == *exempt_id)
                .unwrap_or_else(|| panic!("exempted theme '{exempt_id}' not found in catalog"));
            let match_highlight_raw = theme
                .colors
                .get("panel.matchHighlight")
                .unwrap_or_else(|| panic!("'{exempt_id}' has no panel.matchHighlight"));
            let active_background_raw = theme
                .colors
                .get("list.activeBackground")
                .unwrap_or_else(|| panic!("'{exempt_id}' has no list.activeBackground"));
            let ratio = contrast_ratio(match_highlight_raw, active_background_raw)
                .unwrap_or_else(|| panic!("'{exempt_id}' panel.matchHighlight/list.activeBackground hex 파싱 실패"));

            assert!(
                ratio < MIN_CONTRAST_RATIO,
                "'{exempt_id}' is listed in SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS but its contrast {ratio:.2} already meets MIN_CONTRAST_RATIO — remove the exemption",
            );
        }
    }

    /// `list.foreground` against `list.activeBackground` — the row's non-matched text, same render-path
    /// as `SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS` above but for ordinary row text instead of
    /// search/palette match glyphs. Mirrors TS `SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS`
    /// (`bundled-theme-contrast.test.ts`), which is likewise empty now.
    ///
    /// `rose-pine-dawn` was the one entry, exempted because the gate read that theme's near-transparent
    /// `list.activeBackground` (`#6e6a8614`) at its raw RGB. d-61's review (finding A-1) made the
    /// sibling pairs composite both tokens over their surface, which moved the token to an opaque
    /// `#ece6e3` and lifted the real ratio to 3.56 — so the exemption's premise is gone and keeping a
    /// passing theme listed would disable the lint for it. Kept as an empty registry rather than
    /// deleted so the lint and its reverse check below stay wired for the next genuine case.
    const SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS: &[(&str, &str)] = &[];

    /// WCAG contrast gate (3) for `list.foreground` vs `list.activeBackground` — the general (non-match)
    /// text counterpart to `카탈로그_테마는_panel_매치_하이라이트가_선택_행_배경과_최소_대비를_가진다` above. See
    /// `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-c.
    #[test]
    fn 카탈로그_테마는_list_전경색이_선택_행_배경과_최소_대비를_가진다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            if SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS.iter().any(|(id, _)| *id == theme.id) {
                continue;
            }
            let Some(list_foreground_raw) = theme.colors.get("list.foreground") else {
                continue;
            };
            let Some(active_background_raw) = theme.colors.get("list.activeBackground") else {
                continue;
            };

            match contrast_ratio(list_foreground_raw, active_background_raw) {
                Some(ratio) if ratio >= MIN_CONTRAST_RATIO => {}
                Some(ratio) => violations.push(format!(
                    "'{}': list.foreground({list_foreground_raw:?}) vs list.activeBackground({active_background_raw:?}) = {ratio:.2} (최소 {MIN_CONTRAST_RATIO})",
                    theme.id
                )),
                None => violations.push(format!(
                    "'{}': list.foreground({list_foreground_raw:?}) 또는 list.activeBackground({active_background_raw:?}) hex 파싱 실패",
                    theme.id
                )),
            }
        }

        assert!(
            violations.is_empty(),
            "selection-row list.foreground contrast defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn 선택_행_전경색_대비_예외_등재분은_실제로_최소_대비에_미달한다() {
        let catalog = theme_catalog();

        for (exempt_id, _reason) in SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS {
            let theme = catalog
                .iter()
                .find(|theme| theme.id == *exempt_id)
                .unwrap_or_else(|| panic!("exempted theme '{exempt_id}' not found in catalog"));
            let list_foreground_raw = theme
                .colors
                .get("list.foreground")
                .unwrap_or_else(|| panic!("'{exempt_id}' has no list.foreground"));
            let active_background_raw = theme
                .colors
                .get("list.activeBackground")
                .unwrap_or_else(|| panic!("'{exempt_id}' has no list.activeBackground"));
            let ratio = contrast_ratio(list_foreground_raw, active_background_raw)
                .unwrap_or_else(|| panic!("'{exempt_id}' list.foreground/list.activeBackground hex 파싱 실패"));

            assert!(
                ratio < MIN_CONTRAST_RATIO,
                "'{exempt_id}' is listed in SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS but its contrast {ratio:.2} already meets MIN_CONTRAST_RATIO — remove the exemption",
            );
        }
    }

    /// Identical-color lint for `list.foreground` (post-d40 review findings
    /// d40-listfg-multisurface-regression/d40-l2-01/D40-L3-01) — same shape as
    /// `카탈로그_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다` above, but for the row's *text*
    /// color rather than its selection-highlight fill. `list.foreground` is not selection-only: it is
    /// also the plain row color over `list.background`, and (via `global.css`'s
    /// `--accent-foreground: var(--taide-list-foreground)` on `--accent: var(--taide-list-hover-background)`)
    /// the hover/focus text color for every `hover:text-accent-foreground`/`focus:text-accent-foreground`
    /// consumer (dropdown/context menus, ghost/outline buttons). No exemptions — a catalog theme with
    /// either collapse renders unreadable text on a surface most users hit far more often than the
    /// selected row itself.
    #[test]
    fn 카탈로그_테마는_list_전경색이_list_배경_및_hover_배경과_동일하지_않다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            let list_foreground_raw = theme.colors.get("list.foreground");
            let list_background_raw = theme.colors.get("list.background");
            let list_hover_background_raw = theme.colors.get("list.hoverBackground");

            let list_foreground = list_foreground_raw.map(|value| normalize_hex_color(value));
            let list_background = list_background_raw.map(|value| normalize_hex_color(value));
            let list_hover_background = list_hover_background_raw.map(|value| normalize_hex_color(value));

            if list_foreground == list_background {
                violations.push(format!(
                    "'{}': list.foreground({list_foreground_raw:?}) == list.background({list_background_raw:?}) — the unselected row's text would be invisible",
                    theme.id
                ));
            }
            if list_foreground == list_hover_background {
                violations.push(format!(
                    "'{}': list.foreground({list_foreground_raw:?}) == list.hoverBackground({list_hover_background_raw:?}) — hovering a row (or a --accent menu/button) would make its text disappear",
                    theme.id
                ));
            }
        }

        assert!(
            violations.is_empty(),
            "list.foreground identical-color defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    /// Opacity lint for `list.foreground` — same shape as
    /// `카탈로그_테마는_panel_매치_하이라이트가_불투명하다` above, guarding the other half of the
    /// foreground-opacity fact `hex_to_rgb`'s doc comment relies on (post-d40 review finding
    /// d40-listfg-opacity-ungated): both `panel.matchHighlight` and `list.foreground` must stay
    /// 6-digit opaque hex for `contrast_ratio`'s un-composited foreground read to match TS's
    /// `foregroundContrastRatio` (which *would* composite an alpha-carrying foreground). Before this
    /// lint, only one of the two keys the doc comment cites was actually enforced.
    #[test]
    fn 카탈로그_테마는_list_전경색이_불투명하다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            let Some(list_foreground_raw) = theme.colors.get("list.foreground") else {
                continue;
            };
            let normalized = normalize_hex_color(list_foreground_raw);

            if !normalized.ends_with("ff") {
                violations.push(format!(
                    "'{}': list.foreground({list_foreground_raw:?}) is translucent — this token renders as foreground text (row/palette label), so a translucent value gets absorbed by whatever sits behind it instead of composing a legible color",
                    theme.id
                ));
            }
        }

        assert!(
            violations.is_empty(),
            "list.foreground opacity defects in catalog themes:\n{}",
            violations.join("\n")
        );
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
        let ts_source = include_str!("../../../src/entities/theme/theme-tokens.ts");

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

    /// Minimum CIE76 ΔE*ab a state color must clear against the container it is painted on, mirrored
    /// from TS `STATE_MIN_DISTINCT_DELTA_E` (`src/shared/lib/theme-convert/state-distinctness-pairs.ts`)
    /// — the same 2.3 just-noticeable-difference value `MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E`
    /// (`mapping-tables.ts`) already uses, so every distinctness check in the codebase agrees on what
    /// "a different color" means. `상태색_구별성_쌍_표는_ts_정본과_일치한다` below reads the number back out
    /// of that TS file and compares it, so the two cannot drift apart silently.
    const STATE_MIN_DISTINCT_DELTA_E: f64 = 2.3;

    /// Relaxed counterpart of [`STATE_MIN_DISTINCT_DELTA_E`] for the editor overlays upstream themes
    /// deliberately keep faint (current line, inactive selection, secondary find matches) — mirrored
    /// from TS `SUBTLE_STATE_MIN_DISTINCT_DELTA_E`. Rejects only a state that has collapsed into its
    /// container outright.
    const SUBTLE_STATE_MIN_DISTINCT_DELTA_E: f64 = 1.0;

    /// Minimum alpha `app.shadow` must carry for the separation it exists to draw to survive,
    /// mirrored from TS `APP_SHADOW_MIN_ALPHA` (`state-distinctness-pairs.ts`) — VS Code's own light
    /// `widget.shadow` default (`#00000026`, 38/255), i.e. the faintest shadow the platform itself
    /// ships rather than a value fitted to this catalog. The token colors every floating surface's
    /// drop shadow and, at half strength, the modal scrim, so a zeroed alpha pastes a dialog flat
    /// onto the content behind it with nothing marking it as modal.
    /// `상태색_구별성_쌍_표는_ts_정본과_일치한다` below reads the number back out of that TS file and
    /// compares it, so the two cannot drift apart silently.
    const APP_SHADOW_MIN_ALPHA: f64 = 0.149;

    /// The token [`APP_SHADOW_MIN_ALPHA`] bounds. Mirrors TS `SHADOW_KEY` (`state-distinctness.ts`).
    const APP_SHADOW_KEY: &str = "app.shadow";

    /// One row of [`STATE_DISTINCTNESS_PAIRS`] — a state color, the container it is drawn on, the
    /// opaque surface that container is itself drawn on (so a translucent container is composited
    /// before it is measured), an optional second token allowed to carry the distinction instead,
    /// whether the container is a *sibling* state rather than the surface the state is painted on
    /// (see [`resolve_pair_surfaces`]), and the ΔE the pair must clear.
    struct StateDistinctnessPair {
        label: &'static str,
        state_key: &'static str,
        container_key: &'static str,
        surface_key: Option<&'static str>,
        alternative_state_key: Option<&'static str>,
        sibling: bool,
        min_delta_e: f64,
    }

    /// Verbatim mirror of TS `STATE_DISTINCTNESS_PAIRS`
    /// (`src/shared/lib/theme-convert/state-distinctness-pairs.ts`), which is the single source of
    /// truth: that file derives each pair from the component that renders it and documents the
    /// provenance of every row, and `상태색_구별성_쌍_표는_ts_정본과_일치한다` below re-parses it and
    /// compares label, both keys, the surface, the alternative, the sibling flag and the threshold of
    /// every row in order — the same arrangement `테마_토큰_목록은_rust와_theme_tokens_ts에서_일치한다` uses for the
    /// semantic token list. Rows carry no per-row comment here on purpose: duplicating the prose
    /// would give a second place for it to go stale, while the drift test cannot check prose.
    const STATE_DISTINCTNESS_PAIRS: &[StateDistinctnessPair] = &[
        StateDistinctnessPair {
            label: "editorSelection",
            state_key: "editor.selection",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "editorInactiveSelection",
            state_key: "editor.inactiveSelection",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "editorCurrentLine",
            state_key: "editor.lineHighlight",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "editorFindMatch",
            state_key: "editor.findMatch",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "editorFindMatchHighlight",
            state_key: "editor.findMatchHighlight",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "editorBracketMatch",
            state_key: "editor.bracketMatch",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "editorCursor",
            state_key: "editor.cursor",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "terminalSelection",
            state_key: "terminal.selection",
            container_key: "terminal.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "terminalCursor",
            state_key: "terminal.cursor",
            container_key: "terminal.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "listHover",
            state_key: "list.hoverBackground",
            container_key: "list.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "listActive",
            state_key: "list.activeBackground",
            container_key: "list.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "listActiveVsHover",
            state_key: "list.activeBackground",
            container_key: "list.hoverBackground",
            surface_key: Some("list.background"),
            alternative_state_key: None,
            sibling: true,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "listHoverOnActiveTab",
            state_key: "list.hoverBackground",
            container_key: "tabBar.tabActiveBackground",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "listHoverOnInactiveTab",
            state_key: "list.hoverBackground",
            container_key: "tabBar.tabInactiveBackground",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerItemHover",
            state_key: "explorer.itemHover",
            container_key: "explorer.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerItemSelected",
            state_key: "explorer.itemSelected",
            container_key: "explorer.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerItemFocused",
            state_key: "explorer.itemFocused",
            container_key: "explorer.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerSelectedVsHover",
            state_key: "explorer.itemSelected",
            container_key: "explorer.itemHover",
            surface_key: Some("explorer.background"),
            alternative_state_key: None,
            sibling: true,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerHoverOnPanel",
            state_key: "explorer.itemHover",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerSelectedOnPanel",
            state_key: "explorer.itemSelected",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerFocusedOnPanel",
            state_key: "explorer.itemFocused",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerHoverOnStatusBar",
            state_key: "explorer.itemHover",
            container_key: "appSidebar.background",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerSelectedOnStatusBar",
            state_key: "explorer.itemSelected",
            container_key: "appSidebar.background",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "explorerHoverOnEditorWidget",
            state_key: "explorer.itemHover",
            container_key: "editor.widgetBackground",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarItemHover",
            state_key: "appSidebar.itemHover",
            container_key: "appSidebar.background",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarItemActive",
            state_key: "appSidebar.itemActive",
            container_key: "appSidebar.background",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarActiveVsHover",
            state_key: "appSidebar.itemActive",
            container_key: "appSidebar.itemHover",
            surface_key: Some("appSidebar.background"),
            alternative_state_key: None,
            sibling: true,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarActiveOnCard",
            state_key: "appSidebar.itemActive",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarHoverOnCard",
            state_key: "appSidebar.itemHover",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarHoverOnEditor",
            state_key: "appSidebar.itemHover",
            container_key: "editor.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarHoverOnApp",
            state_key: "appSidebar.itemHover",
            container_key: "app.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarActiveOnApp",
            state_key: "appSidebar.itemActive",
            container_key: "app.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarHoverOnTabBar",
            state_key: "appSidebar.itemHover",
            container_key: "tabBar.background",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "sidebarBadge",
            state_key: "appSidebar.badge",
            container_key: "appSidebar.background",
            surface_key: Some("app.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "tabActive",
            state_key: "tabBar.tabActiveBackground",
            container_key: "tabBar.background",
            surface_key: Some("app.background"),
            alternative_state_key: Some("tabBar.tabActiveIndicator"),
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "tabActiveVsInactive",
            state_key: "tabBar.tabActiveBackground",
            container_key: "tabBar.tabInactiveBackground",
            surface_key: Some("app.background"),
            alternative_state_key: Some("tabBar.tabActiveIndicator"),
            sibling: true,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "switchCheckedTrackOnCard",
            state_key: "button.primaryBackground",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "switchCheckedTrackVsThumb",
            state_key: "button.primaryBackground",
            container_key: "app.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "switchUncheckedTrackOnCard",
            state_key: "input.border",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "switchUncheckedTrackVsThumb",
            state_key: "input.border",
            container_key: "app.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "switchCheckedVsUncheckedTrack",
            state_key: "button.primaryBackground",
            container_key: "input.border",
            surface_key: Some("panel.background"),
            alternative_state_key: None,
            sibling: true,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "buttonHover",
            state_key: "button.hoverBackground",
            container_key: "button.background",
            surface_key: Some("panel.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "inputBorder",
            state_key: "input.border",
            container_key: "input.background",
            surface_key: Some("panel.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "panelInputBorder",
            state_key: "panel.inputBorder",
            container_key: "panel.inputBackground",
            surface_key: Some("panel.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "focusBorderOnApp",
            state_key: "app.focusBorder",
            container_key: "app.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "focusBorderOnCard",
            state_key: "app.focusBorder",
            container_key: "panel.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "inputFocusBorder",
            state_key: "input.focusBorder",
            container_key: "input.background",
            surface_key: Some("panel.background"),
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "menuItemHover",
            state_key: "menu.itemHover",
            container_key: "menu.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "modalItemHover",
            state_key: "modal.itemHover",
            container_key: "modal.background",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
        StateDistinctnessPair {
            label: "scrollbarThumb",
            state_key: "scrollbar.thumb",
            container_key: "scrollbar.track",
            surface_key: None,
            alternative_state_key: None,
            sibling: false,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        },
    ];

    /// One row of [`COMPONENT_CONTRAST_PAIRS`]. `surface_key` is required rather than optional for
    /// the same reason it is on the TS side: a background carrying alpha is the normal case in this
    /// catalog, so every pair names what sits underneath and the background is composited before the
    /// ratio is taken.
    struct ComponentContrastPair {
        label: &'static str,
        foreground_key: &'static str,
        background_key: &'static str,
        surface_key: &'static str,
    }

    /// Verbatim mirror of TS `COMPONENT_CONTRAST_PAIRS`
    /// (`src/shared/lib/theme-convert/component-contrast-pairs.ts`) — every (text color, surface it
    /// is drawn on) pair the UI actually renders, kept in sync by
    /// `컴포넌트_대비_쌍_표는_ts_정본과_일치한다` below. Deliberately separate from the five blocking and
    /// two advisory pairs the lints above guard (`contrast.ts`'s `CONTRAST_PAIRS`): those decide
    /// whether a VSIX import is rejected and have per-theme exemption registries pinned to them,
    /// which is why d-61 §1.B added rows here instead of widening that array.
    const COMPONENT_CONTRAST_PAIRS: &[ComponentContrastPair] = &[
        ComponentContrastPair {
            label: "listRow",
            foreground_key: "list.foreground",
            background_key: "list.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "listHoverRow",
            foreground_key: "list.foreground",
            background_key: "list.hoverBackground",
            surface_key: "list.background",
        },
        ComponentContrastPair {
            label: "explorerGitAdded",
            foreground_key: "explorer.gitAdded",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "explorerGitModified",
            foreground_key: "explorer.gitModified",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "explorerGitDeleted",
            foreground_key: "explorer.gitDeleted",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "explorerGitUntracked",
            foreground_key: "explorer.gitUntracked",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "gitAdded",
            foreground_key: "git.added",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "gitModified",
            foreground_key: "git.modified",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "gitDeleted",
            foreground_key: "git.deleted",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "gitRenamed",
            foreground_key: "git.renamed",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "gitUntracked",
            foreground_key: "git.untracked",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "gitConflicted",
            foreground_key: "git.conflicted",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "gitStaged",
            foreground_key: "git.staged",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "statusBarText",
            foreground_key: "appSidebar.iconDefault",
            background_key: "appSidebar.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "panelMutedText",
            foreground_key: "appSidebar.iconDefault",
            background_key: "panel.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "explorerMutedText",
            foreground_key: "appSidebar.iconDefault",
            background_key: "explorer.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "panelBadge",
            foreground_key: "appSidebar.badge",
            background_key: "panel.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "statusBarError",
            foreground_key: "statusIndicator.error",
            background_key: "appSidebar.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "statusBarWarning",
            foreground_key: "statusIndicator.warning",
            background_key: "appSidebar.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "statusBarSuccess",
            foreground_key: "statusIndicator.success",
            background_key: "appSidebar.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "problemError",
            foreground_key: "statusIndicator.error",
            background_key: "panel.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "problemWarning",
            foreground_key: "statusIndicator.warning",
            background_key: "panel.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "problemInfo",
            foreground_key: "statusIndicator.info",
            background_key: "panel.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "problemSuccess",
            foreground_key: "statusIndicator.success",
            background_key: "panel.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "inputText",
            foreground_key: "input.foreground",
            background_key: "input.background",
            surface_key: "editor.widgetBackground",
        },
        ComponentContrastPair {
            label: "inputPlaceholder",
            foreground_key: "input.placeholder",
            background_key: "input.background",
            surface_key: "editor.widgetBackground",
        },
        ComponentContrastPair {
            label: "buttonPrimary",
            foreground_key: "button.primaryForeground",
            background_key: "button.primaryBackground",
            surface_key: "panel.background",
        },
        ComponentContrastPair {
            label: "buttonSecondary",
            foreground_key: "button.foreground",
            background_key: "button.background",
            surface_key: "panel.background",
        },
        ComponentContrastPair {
            label: "buttonSecondaryHover",
            foreground_key: "button.foreground",
            background_key: "button.hoverBackground",
            surface_key: "panel.background",
        },
        ComponentContrastPair {
            label: "tabActive",
            foreground_key: "tabBar.tabActiveForeground",
            background_key: "tabBar.tabActiveBackground",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "tabInactive",
            foreground_key: "tabBar.tabInactiveForeground",
            background_key: "tabBar.tabInactiveBackground",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "tabPreview",
            foreground_key: "tabBar.previewForeground",
            background_key: "tabBar.tabActiveBackground",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "terminalText",
            foreground_key: "terminal.foreground",
            background_key: "terminal.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "editorLink",
            foreground_key: "terminal.linkForeground",
            background_key: "editor.background",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "editorWidgetLink",
            foreground_key: "terminal.linkForeground",
            background_key: "editor.widgetBackground",
            surface_key: "app.background",
        },
        ComponentContrastPair {
            label: "menuItemHoverText",
            foreground_key: "app.foreground",
            background_key: "menu.itemHover",
            surface_key: "menu.background",
        },
    ];

    /// Verbatim mirror of TS `COMPONENT_CONTRAST_EXEMPTIONS`
    /// (`src/shared/lib/theme-convert/component-contrast-pairs.ts`), keyed `<theme id>:<pair label>`:
    /// the catalog themes that stay below [`MIN_CONTRAST_RATIO`] on one axis of
    /// [`COMPONENT_CONTRAST_PAIRS`] even after repair. Both entries are ayu's, and both are the same
    /// shape — the theme's UI foreground already sits at the contrast floor against the plain menu
    /// background (3.09:1 dark / 3.32:1 light), so any hover tint that reads as a hover at all spends
    /// the remaining margin, and the only other lever is the global body color the repair rule never
    /// moves. `컴포넌트_대비_예외_등재분은_실제로_최소_대비에_미달한다` below re-measures every entry, so an
    /// exemption that stops being needed fails the suite instead of lingering.
    const COMPONENT_CONTRAST_EXEMPTIONS: &[(&str, &str)] = &[("ayu-dark", "menuItemHoverText"), ("ayu-light", "menuItemHoverText")];

    /// CIE 1976 L*a*b* (D65, 2° standard observer) constants for [`delta_e76`], ported verbatim from
    /// TS `shared/lib/color.ts`. The linearization breakpoint here is the sRGB color-science value
    /// (0.04045, IEC 61966-2-1), deliberately not the WCAG relative-luminance breakpoint
    /// [`SRGB_LINEAR_THRESHOLD`] (0.03928) the contrast helpers above use — the two measure different
    /// things (perceptual distance vs contrast ratio) and TS keeps the same split.
    const CIE_SRGB_LINEAR_THRESHOLD: f64 = 0.04045;
    const CIE_SRGB_LINEAR_DIVISOR: f64 = 12.92;
    const CIE_SRGB_GAMMA_OFFSET: f64 = 0.055;
    const CIE_SRGB_GAMMA_DIVISOR: f64 = 1.055;
    const CIE_SRGB_GAMMA_EXPONENT: f64 = 2.4;
    const CIE_XYZ_MATRIX_X: [f64; 3] = [0.4124564, 0.3575761, 0.1804375];
    const CIE_XYZ_MATRIX_Y: [f64; 3] = [0.2126729, 0.7151522, 0.072175];
    const CIE_XYZ_MATRIX_Z: [f64; 3] = [0.0193339, 0.119192, 0.9503041];
    const CIE_D65_WHITE_POINT: [f64; 3] = [0.95047, 1.0, 1.08883];
    const CIE_LAB_EPSILON: f64 = 216.0 / 24389.0;
    const CIE_LAB_KAPPA: f64 = 24389.0 / 27.0;
    const CIE_LAB_L_SCALE: f64 = 116.0;
    const CIE_LAB_L_OFFSET: f64 = 16.0;
    const CIE_LAB_A_SCALE: f64 = 500.0;
    const CIE_LAB_B_SCALE: f64 = 200.0;

    /// Length of an `#rrggbbaa` value, the only form [`composite_over_background`] composites — same
    /// rule as TS `compositeOverBackground`, which passes 3-, 6- and 8-digit forms through untouched.
    const HEX_WITH_ALPHA_LENGTH: usize = 9;
    const OPAQUE_HEX_LENGTH: usize = 7;
    const ALPHA_CHANNEL_MAX: f64 = 255.0;
    const HEX_RADIX: u32 = 16;

    /// The root surface every other surface is ultimately painted on — mirrors TS
    /// `APP_BACKGROUND_KEY` (`contrast.ts`). No catalog theme gives it an alpha channel, which is
    /// what lets [`resolve_component_surface`] resolve a translucent surface against something
    /// concrete instead of recursing.
    const APP_BACKGROUND_KEY: &str = "app.background";

    /// Ports TS `isHexColor` (`shared/lib/color.ts`) — the shape check every measurement below uses
    /// to decide whether a token is measurable at all. A value that is missing, `transparent`, or an
    /// unresolved `$palette` reference makes its pair unmeasurable rather than violating, exactly as
    /// on the TS side.
    fn is_hex_color(value: &str) -> bool {
        let trimmed = value.trim();
        let Some(digits) = trimmed.strip_prefix('#') else {
            return false;
        };
        matches!(digits.len(), 3 | 6 | 8) && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    /// Ports TS `toHexChannel` (`shared/lib/color.ts`) — clamp, round, lowercase two-digit hex.
    fn hex_channel(channel: f64) -> String {
        format!("{:02x}", channel.clamp(0.0, RGB_CHANNEL_MAX).round() as u8)
    }

    /// Ports TS `rgbToHex` (`shared/lib/color.ts`).
    fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
        format!("#{}{}{}", hex_channel(r), hex_channel(g), hex_channel(b))
    }

    /// The composition half of [`composite_over_background`], split out so the fallback path (a
    /// malformed value that got past the length check) stays a plain `unwrap_or_else` instead of a
    /// nest of early returns.
    fn composite_channels(foreground: &str, background: &str, opaque: &str) -> Option<String> {
        let (foreground_r, foreground_g, foreground_b) = hex_to_rgb(opaque)?;
        let (background_r, background_g, background_b) = hex_to_rgb(background)?;
        let alpha_digits = foreground.get(OPAQUE_HEX_LENGTH..HEX_WITH_ALPHA_LENGTH)?;
        let alpha = f64::from(u8::from_str_radix(alpha_digits, HEX_RADIX).ok()?) / ALPHA_CHANNEL_MAX;
        Some(rgb_to_hex(
            foreground_r * alpha + background_r * (1.0 - alpha),
            foreground_g * alpha + background_g * (1.0 - alpha),
            foreground_b * alpha + background_b * (1.0 - alpha),
        ))
    }

    /// Ports TS `compositeOverBackground` (`shared/lib/color.ts`) — the opaque color an
    /// `#rrggbbaa` value actually renders as over a given background. Every other hex form passes
    /// through unchanged, which makes this an identity operation for opaque tokens. The lints below
    /// composite both the state/foreground *and* the container/background, because a token that
    /// scores against its raw RGB is being measured as a color nothing on screen ever shows.
    fn composite_over_background(foreground: &str, background: &str) -> String {
        if foreground.len() != HEX_WITH_ALPHA_LENGTH {
            return foreground.to_string();
        }
        let opaque = foreground.get(..OPAQUE_HEX_LENGTH).unwrap_or(foreground);
        composite_channels(foreground, background, opaque).unwrap_or_else(|| opaque.to_string())
    }

    /// Ports TS `srgbChannelToLinear`'s color-science twin in `shared/lib/color.ts`
    /// (`srgbChannelToCieLinear`) — the sRGB electro-optical transfer function the L*a*b*
    /// conversion needs.
    fn srgb_channel_to_cie_linear(channel: f64) -> f64 {
        let normalized = channel / RGB_CHANNEL_MAX;
        if normalized <= CIE_SRGB_LINEAR_THRESHOLD {
            normalized / CIE_SRGB_LINEAR_DIVISOR
        } else {
            ((normalized + CIE_SRGB_GAMMA_OFFSET) / CIE_SRGB_GAMMA_DIVISOR).powf(CIE_SRGB_GAMMA_EXPONENT)
        }
    }

    /// Ports TS `cieLabF` (`shared/lib/color.ts`) — the CIE's piecewise f(t) helper.
    fn cie_lab_f(t: f64) -> f64 {
        if t > CIE_LAB_EPSILON {
            t.cbrt()
        } else {
            (CIE_LAB_KAPPA * t + CIE_LAB_L_OFFSET) / CIE_LAB_L_SCALE
        }
    }

    /// Ports TS `hexToLab` (`shared/lib/color.ts`). Reads only the RGB half of an alpha-carrying
    /// value, same as [`hex_to_rgb`] above — callers composite first when what matters is the color
    /// on screen.
    fn hex_to_lab(value: &str) -> Option<(f64, f64, f64)> {
        let (r, g, b) = hex_to_rgb(value)?;
        let r_linear = srgb_channel_to_cie_linear(r);
        let g_linear = srgb_channel_to_cie_linear(g);
        let b_linear = srgb_channel_to_cie_linear(b);
        let x = r_linear * CIE_XYZ_MATRIX_X[0] + g_linear * CIE_XYZ_MATRIX_X[1] + b_linear * CIE_XYZ_MATRIX_X[2];
        let y = r_linear * CIE_XYZ_MATRIX_Y[0] + g_linear * CIE_XYZ_MATRIX_Y[1] + b_linear * CIE_XYZ_MATRIX_Y[2];
        let z = r_linear * CIE_XYZ_MATRIX_Z[0] + g_linear * CIE_XYZ_MATRIX_Z[1] + b_linear * CIE_XYZ_MATRIX_Z[2];
        let fx = cie_lab_f(x / CIE_D65_WHITE_POINT[0]);
        let fy = cie_lab_f(y / CIE_D65_WHITE_POINT[1]);
        let fz = cie_lab_f(z / CIE_D65_WHITE_POINT[2]);
        Some((
            CIE_LAB_L_SCALE * fy - CIE_LAB_L_OFFSET,
            CIE_LAB_A_SCALE * (fx - fy),
            CIE_LAB_B_SCALE * (fy - fz),
        ))
    }

    /// Ports TS `deltaE76` (`shared/lib/color.ts`) — straight-line distance in L*a*b* space. Unlike
    /// [`contrast_ratio`] this measures how *distinguishable* two colors are regardless of which is
    /// lighter, which is what a state-vs-container comparison needs: a selection band and the text
    /// canvas behind it can be equally luminant and still obviously different.
    fn delta_e76(hex_a: &str, hex_b: &str) -> Option<f64> {
        let (l_a, a_a, b_a) = hex_to_lab(hex_a)?;
        let (l_b, a_b, b_b) = hex_to_lab(hex_b)?;
        Some(((l_a - l_b).powi(2) + (a_a - a_b).powi(2) + (b_a - b_b).powi(2)).sqrt())
    }

    /// Ports TS `foregroundContrastRatio` (`contrast.ts`) — the WCAG ratio between a foreground as
    /// it renders over a background and that background. Distinct from [`contrast_ratio`] above,
    /// which the five pre-existing pairs use on raw values and which stays untouched
    /// (`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-a).
    fn foreground_contrast_ratio(foreground: &str, background: &str) -> Option<f64> {
        contrast_ratio(&composite_over_background(foreground, background), background)
    }

    /// Ports TS `resolvePairSurfaces` (`state-distinctness.ts`) — the two opaque colors a pair is
    /// compared at: the container as it renders, and the base the state is composited over before the
    /// comparison.
    ///
    /// A container carrying alpha is composited over the pair's surface first; without that,
    /// `dracula`'s `explorer.itemHover` (`#44475A75`, an overlay of the very color
    /// `explorer.itemSelected` uses opaquely) would score ΔE 0 against a state that is plainly
    /// distinguishable on screen. For a normal pair the state is painted *on* the container, so its
    /// base is that resolved container; for a [`StateDistinctnessPair::sibling`] pair the two tokens
    /// are peers on the same surface, so both are composited over that surface independently — which
    /// is what catches ayu's `explorer.itemSelected` and `explorer.itemHover` shipping the same
    /// overlay (laying one over the other tints it twice and scores ΔE 6+ for two rows that render
    /// identically).
    fn resolve_pair_surfaces(pair: &StateDistinctnessPair, colors: &BTreeMap<String, String>) -> Option<(String, String)> {
        let container = colors.get(pair.container_key)?;
        if !is_hex_color(container) {
            return None;
        }
        let surface = pair.surface_key.and_then(|key| colors.get(key)).filter(|value| is_hex_color(value));
        let container_hex = match surface {
            Some(surface) => composite_over_background(container, surface),
            None => container.clone(),
        };
        let state_base_hex = match (pair.sibling, surface) {
            (true, Some(surface)) => surface.clone(),
            _ => container_hex.clone(),
        };
        Some((container_hex, state_base_hex))
    }

    /// Ports TS `measureDistance` (`state-distinctness.ts`) — the state composited over its base,
    /// then compared with the container.
    fn state_distance(state: &str, container_hex: &str, state_base_hex: &str) -> Option<f64> {
        if !is_hex_color(state) {
            return None;
        }
        delta_e76(&composite_over_background(state, state_base_hex), container_hex)
    }

    /// Ports TS `shadowAlpha` (`state-distinctness.ts`) — the alpha `app.shadow` renders at. A
    /// 6-digit value is opaque; a value that is not a hex color at all (`transparent`, an unresolved
    /// `@palette` reference) returns `None` and is left alone, the same "unmeasurable rather than
    /// violating" rule the pairs follow.
    fn shadow_alpha(shadow: &str) -> Option<f64> {
        if !is_hex_color(shadow) {
            return None;
        }
        if shadow.len() != HEX_WITH_ALPHA_LENGTH {
            return Some(1.0);
        }
        let alpha_digits = shadow.get(OPAQUE_HEX_LENGTH..HEX_WITH_ALPHA_LENGTH)?;
        u8::from_str_radix(alpha_digits, HEX_RADIX)
            .ok()
            .map(|alpha| f64::from(alpha) / ALPHA_CHANNEL_MAX)
    }

    /// Ports TS `resolveComponentSurface` (`contrast.ts`) — a surface is itself composited over
    /// [`APP_BACKGROUND_KEY`], which is what makes a translucent surface (`darcula`'s
    /// `appSidebar.background`, `#ffffff1a`) resolve to what the user sees.
    fn resolve_component_surface(colors: &BTreeMap<String, String>, surface_key: &str) -> Option<String> {
        let surface = colors.get(surface_key)?;
        if !is_hex_color(surface) {
            return None;
        }
        if surface_key == APP_BACKGROUND_KEY {
            return Some(surface.clone());
        }
        let Some(root) = colors.get(APP_BACKGROUND_KEY).filter(|value| is_hex_color(value)) else {
            return Some(surface.clone());
        };
        Some(composite_over_background(surface, root))
    }

    /// Ports TS `resolveComponentBackground` (`contrast.ts`).
    fn resolve_component_background(pair: &ComponentContrastPair, colors: &BTreeMap<String, String>) -> Option<String> {
        let background = colors.get(pair.background_key)?;
        if !is_hex_color(background) {
            return None;
        }
        match resolve_component_surface(colors, pair.surface_key) {
            Some(surface) => Some(composite_over_background(background, &surface)),
            None => Some(background.clone()),
        }
    }

    /// Rust mirror of the TS catalog gate `bundled-theme-state-distinctness.test.ts`
    /// (d-61 §1.A). Carries no exemption registry, and unlike the TS gate it also covers the two
    /// Rust-literal builtins via [`theme_catalog`] — the reason
    /// `docs/acknowledge/2026-08-25-d36-theme-catalog-audit-contract.md` §1-b widened every other
    /// lint in this module the same way, after a `taide-light` defect shipped because
    /// `bundled_themes()`-only lints never looked at it. Every axis here is repairable by
    /// construction (`repairStateDistinctness` walks the container toward the theme's own body
    /// foreground), so a failure means `bun run themes:repair-state-distinctness` was not run, not
    /// that a theme legitimately collapses the state.
    ///
    /// Covers the pair table plus the one axis that has no container to be compared against
    /// ([`APP_SHADOW_MIN_ALPHA`], d-61 review finding G-2): `app.shadow` is drawn over every surface
    /// in the app, so what makes it collapse is its own alpha rather than a ΔE against another token.
    #[test]
    fn 카탈로그_테마는_상태색이_바탕색과_구별된다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            for pair in STATE_DISTINCTNESS_PAIRS {
                let (Some(state_raw), Some(container_raw)) = (theme.colors.get(pair.state_key), theme.colors.get(pair.container_key))
                else {
                    continue;
                };
                let Some((container_hex, state_base_hex)) = resolve_pair_surfaces(pair, &theme.colors) else {
                    continue;
                };
                let Some(distance) = state_distance(state_raw, &container_hex, &state_base_hex) else {
                    continue;
                };
                let alternative = pair
                    .alternative_state_key
                    .and_then(|key| theme.colors.get(key))
                    .and_then(|value| state_distance(value, &container_hex, &state_base_hex))
                    .unwrap_or(0.0);

                if distance >= pair.min_delta_e || alternative >= pair.min_delta_e {
                    continue;
                }
                violations.push(format!(
                    "'{}': {} 구별성 부족: {}({state_raw:?}) vs {}({container_raw:?}) = ΔE {distance:.2} (최소 {})",
                    theme.id, pair.label, pair.state_key, pair.container_key, pair.min_delta_e
                ));
            }

            let Some(shadow) = theme.colors.get(APP_SHADOW_KEY) else {
                continue;
            };
            let Some(alpha) = shadow_alpha(shadow) else {
                continue;
            };
            if alpha < APP_SHADOW_MIN_ALPHA {
                violations.push(format!(
                    "'{}': appShadow 구별성 부족: {APP_SHADOW_KEY}({shadow:?}) 알파 {alpha:.3} (최소 {APP_SHADOW_MIN_ALPHA})",
                    theme.id
                ));
            }
        }

        assert!(
            violations.is_empty(),
            "state distinctness defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    /// Pins the [`StateDistinctnessPair::sibling`] measurement itself, which neither guard around it
    /// can: the catalog lint only reports what the repaired data happens to hit, and
    /// `상태색_구별성_쌍_표는_ts_정본과_일치한다` compares the two tables rather than what they measure, so a
    /// mirror that read the flag and ignored it would pass both. Two peer states shipping the *same*
    /// translucent overlay (ayu's `explorer.itemSelected`/`itemHover` before d-61 repaired them)
    /// render identically; composing one over the other tints it a second time and scores a
    /// comfortable pass, which is the false negative the flag removes.
    #[test]
    fn 형제_쌍은_양쪽을_표면_위에_독립_합성해_비교한다() {
        let colors = map_from_pairs(&[
            ("explorer.background", "#0f131a"),
            ("explorer.itemHover", "#e6b45040"),
            ("explorer.itemSelected", "#e6b45040"),
        ]);
        let sibling_pair = StateDistinctnessPair {
            label: "explorerSelectedVsHover",
            state_key: "explorer.itemSelected",
            container_key: "explorer.itemHover",
            surface_key: Some("explorer.background"),
            alternative_state_key: None,
            sibling: true,
            min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        };
        let stacked_pair = StateDistinctnessPair {
            sibling: false,
            ..sibling_pair
        };

        let measure = |pair: &StateDistinctnessPair| {
            let (container_hex, state_base_hex) = resolve_pair_surfaces(pair, &colors).expect("두 토큰 모두 hex 입니다");
            state_distance(&colors[pair.state_key], &container_hex, &state_base_hex).expect("state 가 hex 입니다")
        };

        assert!(
            measure(&sibling_pair) < STATE_MIN_DISTINCT_DELTA_E,
            "같은 오버레이를 쓰는 형제 상태가 구별된다고 측정됐습니다: ΔE {:.2}",
            measure(&sibling_pair)
        );
        assert!(
            measure(&stacked_pair) >= STATE_MIN_DISTINCT_DELTA_E,
            "형제 플래그 없이 쌓아 재면 통과해야 이 테스트가 플래그를 검증합니다: ΔE {:.2}",
            measure(&stacked_pair)
        );
    }

    /// Rust mirror of the TS catalog gate for `COMPONENT_CONTRAST_PAIRS`
    /// (`bundled-theme-contrast.test.ts`, d-61 §1.B) — the component-wide extension of the
    /// three single-pair contrast lints above. Same threshold ([`MIN_CONTRAST_RATIO`], WCAG 1.4.11's
    /// 3:1 for non-text UI), same full-catalog iteration source: the table excludes the axes where a
    /// single token cannot satisfy every surface it serves (`docs/theme-system.md` §8.6 records those
    /// with their measurements), so every row left in it is repairable by `repairComponentContrast`
    /// and a failure normally means `bun run themes:repair-contrast` was not run.
    ///
    /// The exception is the `menuItemHoverText` axis d-61's review added (finding G-3), whose
    /// foreground *is* the shared body color and which is repaired on the background side instead.
    /// Where even that cannot clear the threshold the theme is listed in
    /// [`COMPONENT_CONTRAST_EXEMPTIONS`], the mirror of the one registry the TS gate and the repair
    /// script read.
    #[test]
    fn 카탈로그_테마는_컴포넌트_전경색이_실제_배경과_최소_대비를_가진다() {
        let mut violations = Vec::new();

        for theme in theme_catalog() {
            for pair in COMPONENT_CONTRAST_PAIRS {
                if COMPONENT_CONTRAST_EXEMPTIONS
                    .iter()
                    .any(|(id, label)| *id == theme.id && *label == pair.label)
                {
                    continue;
                }
                let Some(foreground_raw) = theme.colors.get(pair.foreground_key).filter(|value| is_hex_color(value)) else {
                    continue;
                };
                let Some(background_raw) = theme.colors.get(pair.background_key) else {
                    continue;
                };
                let Some(background) = resolve_component_background(pair, &theme.colors) else {
                    continue;
                };

                match foreground_contrast_ratio(foreground_raw, &background) {
                    Some(ratio) if ratio >= MIN_CONTRAST_RATIO => {}
                    Some(ratio) => violations.push(format!(
                        "'{}': {} 대비 부족: {}({foreground_raw:?}) vs {}({background_raw:?} -> {background}) = {ratio:.2} (최소 {MIN_CONTRAST_RATIO})",
                        theme.id, pair.label, pair.foreground_key, pair.background_key
                    )),
                    None => violations.push(format!(
                        "'{}': {} — {}({foreground_raw:?}) 또는 {}({background_raw:?}) hex 파싱 실패",
                        theme.id, pair.label, pair.foreground_key, pair.background_key
                    )),
                }
            }
        }

        assert!(
            violations.is_empty(),
            "component contrast defects in catalog themes:\n{}",
            violations.join("\n")
        );
    }

    /// The exemption counterpart of the lint above, in the same shape as
    /// `선택_행_전경색_대비_예외_등재분은_실제로_최소_대비에_미달한다`: an entry of
    /// [`COMPONENT_CONTRAST_EXEMPTIONS`] that no longer measures below the threshold is a stale
    /// exemption quietly disabling a lint for a theme that would now pass it, so it has to fail here
    /// rather than linger. The TS gate holds its copy of the registry to the same rule
    /// (`bundled-theme-contrast.test.ts`'s `예외 등재분은 실제로 등재된 축에서만 위반한다`).
    #[test]
    fn 컴포넌트_대비_예외_등재분은_실제로_최소_대비에_미달한다() {
        let catalog = theme_catalog();

        for (exempt_id, exempt_label) in COMPONENT_CONTRAST_EXEMPTIONS {
            let theme = catalog
                .iter()
                .find(|theme| theme.id == *exempt_id)
                .unwrap_or_else(|| panic!("exempted theme '{exempt_id}' not found in catalog"));
            let pair = COMPONENT_CONTRAST_PAIRS
                .iter()
                .find(|pair| pair.label == *exempt_label)
                .unwrap_or_else(|| panic!("'{exempt_id}' 의 예외 축 '{exempt_label}' 이 COMPONENT_CONTRAST_PAIRS 에 없습니다"));
            let foreground_raw = theme
                .colors
                .get(pair.foreground_key)
                .unwrap_or_else(|| panic!("'{exempt_id}' has no {}", pair.foreground_key));
            let background = resolve_component_background(pair, &theme.colors)
                .unwrap_or_else(|| panic!("'{exempt_id}' {} hex 파싱 실패", pair.background_key));
            let ratio = foreground_contrast_ratio(foreground_raw, &background)
                .unwrap_or_else(|| panic!("'{exempt_id}' {} hex 파싱 실패", pair.foreground_key));

            assert!(
                ratio < MIN_CONTRAST_RATIO,
                "'{exempt_id}' is listed in COMPONENT_CONTRAST_EXEMPTIONS for '{exempt_label}' but its contrast {ratio:.2} already meets MIN_CONTRAST_RATIO — remove the exemption",
            );
        }
    }

    /// Extracts the object literals of a `... = [` table in one of the TS mirror sources, in file
    /// order. Entries hold no nested braces (the per-row provenance docs are line comments outside
    /// them), so a non-nesting `{...}` match is exact rather than a heuristic — and if that ever
    /// stops holding, the drift tests below fail loudly with a row count mismatch instead of
    /// silently comparing fewer rows.
    fn ts_table_entries(source: &str, marker: &str) -> Vec<String> {
        let start = source.find(marker).unwrap_or_else(|| panic!("{marker} 를 찾을 수 없습니다")) + marker.len();
        let end = source[start..]
            .find("\n]")
            .unwrap_or_else(|| panic!("{marker} 의 끝(\\n])을 찾을 수 없습니다"));
        let block = &source[start..start + end];
        Regex::new(r"\{([^{}]*)\}")
            .expect("유효한 정규식")
            .captures_iter(block)
            .map(|capture| capture[1].to_string())
            .collect()
    }

    /// Reads a quoted string field out of one TS object literal. The leading `(?:^|[\s,{])` keeps
    /// `stateKey` from also matching inside a longer key that ends with the same letters.
    fn ts_entry_string(entry: &str, name: &str) -> Option<String> {
        Regex::new(&format!(r"(?:^|[\s,{{]){name}:\s*'([^']+)'"))
            .expect("유효한 정규식")
            .captures(entry)
            .map(|capture| capture[1].to_string())
    }

    /// Reads an optional boolean field out of one TS object literal. A row that omits the field is
    /// `false`, matching the TS type's `sibling?: boolean`; `sibling: false` is never written on the
    /// TS side, so an explicit `false` reads back the same way an omission does.
    fn ts_entry_flag(entry: &str, name: &str) -> bool {
        Regex::new(&format!(r"(?:^|[\s,{{]){name}:\s*true"))
            .expect("유효한 정규식")
            .is_match(entry)
    }

    /// Reads a field whose value is a SCREAMING_CASE constant reference rather than a literal.
    fn ts_entry_constant(entry: &str, name: &str) -> Option<String> {
        Regex::new(&format!(r"(?:^|[\s,{{]){name}:\s*([A-Z_]+)"))
            .expect("유효한 정규식")
            .captures(entry)
            .map(|capture| capture[1].to_string())
    }

    /// Reads a numeric `const NAME = <number>` declaration out of a TS source, so the thresholds
    /// mirrored into this module are compared against their originals rather than trusted.
    fn ts_number_constant(source: &str, name: &str) -> f64 {
        Regex::new(&format!(r"(?m)^\s*(?:export )?const {name} = ([0-9.]+)"))
            .expect("유효한 정규식")
            .captures(source)
            .unwrap_or_else(|| panic!("{name} 상수를 찾을 수 없습니다"))[1]
            .parse()
            .expect("숫자 상수")
    }

    /// One comparable line per pair, used for both sides of the state-distinctness drift test so a
    /// mismatch prints as a readable diff of the rows themselves. `sibling` is part of the key
    /// because it changes what the pair measures, not just which tokens it reads: a mirror that kept
    /// the tokens but dropped the flag would compare a state composited over its container against a
    /// TS gate that composites both over the surface, and pass.
    fn describe_state_pair(
        label: &str,
        state_key: &str,
        container_key: &str,
        surface_key: Option<&str>,
        alternative_state_key: Option<&str>,
        sibling: bool,
        min_delta_e: f64,
    ) -> String {
        format!(
            "{label}|{state_key}|{container_key}|{}|{}|{sibling}|{min_delta_e:.4}",
            surface_key.unwrap_or("-"),
            alternative_state_key.unwrap_or("-")
        )
    }

    /// d-61 §1.A's drift guard: [`STATE_DISTINCTNESS_PAIRS`] is hand-mirrored from TS with no
    /// generator, so a row added, reordered, retargeted or re-thresholded on one side would
    /// otherwise leave the Rust lint quietly checking a different table than the TS gate and the
    /// repair script.
    #[test]
    fn 상태색_구별성_쌍_표는_ts_정본과_일치한다() {
        let ts_source = include_str!("../../../src/shared/lib/theme-convert/state-distinctness-pairs.ts");
        let state_threshold = ts_number_constant(ts_source, "STATE_MIN_DISTINCT_DELTA_E");
        let subtle_threshold = ts_number_constant(ts_source, "SUBTLE_STATE_MIN_DISTINCT_DELTA_E");

        assert_eq!(
            state_threshold, STATE_MIN_DISTINCT_DELTA_E,
            "STATE_MIN_DISTINCT_DELTA_E 가 Rust 미러와 TS 정본에서 다릅니다"
        );
        assert_eq!(
            subtle_threshold, SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
            "SUBTLE_STATE_MIN_DISTINCT_DELTA_E 가 Rust 미러와 TS 정본에서 다릅니다"
        );
        assert_eq!(
            ts_number_constant(ts_source, "APP_SHADOW_MIN_ALPHA"),
            APP_SHADOW_MIN_ALPHA,
            "APP_SHADOW_MIN_ALPHA 가 Rust 미러와 TS 정본에서 다릅니다"
        );

        let ts_rows: Vec<String> = ts_table_entries(
            ts_source,
            "export const STATE_DISTINCTNESS_PAIRS: readonly StateDistinctnessPair[] = [",
        )
        .iter()
        .map(|entry| {
            let label = ts_entry_string(entry, "label").unwrap_or_else(|| panic!("label 없는 쌍: {entry}"));
            let threshold = match ts_entry_constant(entry, "minDeltaE").as_deref() {
                Some("STATE_MIN_DISTINCT_DELTA_E") => state_threshold,
                Some("SUBTLE_STATE_MIN_DISTINCT_DELTA_E") => subtle_threshold,
                other => panic!("'{label}' 의 minDeltaE 상수를 알 수 없습니다: {other:?}"),
            };
            describe_state_pair(
                &label,
                &ts_entry_string(entry, "stateKey").unwrap_or_else(|| panic!("'{label}' 에 stateKey 가 없습니다")),
                &ts_entry_string(entry, "containerKey").unwrap_or_else(|| panic!("'{label}' 에 containerKey 가 없습니다")),
                ts_entry_string(entry, "surfaceKey").as_deref(),
                ts_entry_string(entry, "alternativeStateKey").as_deref(),
                ts_entry_flag(entry, "sibling"),
                threshold,
            )
        })
        .collect();

        let rust_rows: Vec<String> = STATE_DISTINCTNESS_PAIRS
            .iter()
            .map(|pair| {
                describe_state_pair(
                    pair.label,
                    pair.state_key,
                    pair.container_key,
                    pair.surface_key,
                    pair.alternative_state_key,
                    pair.sibling,
                    pair.min_delta_e,
                )
            })
            .collect();

        assert!(!ts_rows.is_empty(), "TS 정본에서 상태색 구별성 쌍을 한 건도 읽지 못했습니다");
        assert_eq!(
            rust_rows, ts_rows,
            "상태색 구별성 쌍 표가 Rust 미러와 TS 정본(state-distinctness-pairs.ts) 사이에서 다릅니다"
        );
    }

    /// d-61 §1.B's drift guard, the component-contrast counterpart of
    /// `상태색_구별성_쌍_표는_ts_정본과_일치한다` above. Also pins [`MIN_CONTRAST_RATIO`] to the TS
    /// threshold it claims to mirror.
    #[test]
    fn 컴포넌트_대비_쌍_표는_ts_정본과_일치한다() {
        let ts_source = include_str!("../../../src/shared/lib/theme-convert/component-contrast-pairs.ts");
        let contrast_source = include_str!("../../../src/shared/lib/theme-convert/contrast.ts");

        assert_eq!(
            ts_number_constant(contrast_source, "MIN_CONTRAST_RATIO"),
            MIN_CONTRAST_RATIO,
            "MIN_CONTRAST_RATIO 가 Rust 미러와 TS 정본(contrast.ts)에서 다릅니다"
        );

        let ts_rows: Vec<String> = ts_table_entries(
            ts_source,
            "export const COMPONENT_CONTRAST_PAIRS: readonly ComponentContrastPair[] = [",
        )
        .iter()
        .map(|entry| {
            let label = ts_entry_string(entry, "label").unwrap_or_else(|| panic!("label 없는 쌍: {entry}"));
            let foreground = ts_entry_string(entry, "foregroundKey").unwrap_or_else(|| panic!("'{label}' 에 foregroundKey 가 없습니다"));
            let background = ts_entry_string(entry, "backgroundKey").unwrap_or_else(|| panic!("'{label}' 에 backgroundKey 가 없습니다"));
            let surface = ts_entry_string(entry, "surfaceKey").unwrap_or_else(|| panic!("'{label}' 에 surfaceKey 가 없습니다"));
            format!("{label}|{foreground}|{background}|{surface}")
        })
        .collect();

        let rust_rows: Vec<String> = COMPONENT_CONTRAST_PAIRS
            .iter()
            .map(|pair| {
                format!(
                    "{}|{}|{}|{}",
                    pair.label, pair.foreground_key, pair.background_key, pair.surface_key
                )
            })
            .collect();

        assert!(!ts_rows.is_empty(), "TS 정본에서 컴포넌트 대비 쌍을 한 건도 읽지 못했습니다");
        assert_eq!(
            rust_rows, ts_rows,
            "컴포넌트 대비 쌍 표가 Rust 미러와 TS 정본(component-contrast-pairs.ts) 사이에서 다릅니다"
        );
    }

    /// d-61 §1.D's registration gate. [`BUNDLED_THEME_SOURCES`] is a hand-written `include_str!`
    /// list, so a theme file added to `resources/themes/` without a row here ships as dead weight in
    /// the binary and never appears in `list_themes`, while a row paired with the wrong file makes
    /// `builtin_by_id` answer one id with another theme's colors. Reads the directory through
    /// `CARGO_MANIFEST_DIR` — a source-tree path, which is exactly what a data-quality lint wants;
    /// the TS side gates the same two facts against `THIRD_PARTY_LICENSES.md`
    /// (`bundled-theme-licenses.test.ts`).
    #[test]
    fn 번들_테마_등록_배열은_리소스_디렉터리_및_theme_id_와_일치한다() {
        let themes_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/themes");
        let mut files: Vec<String> = std::fs::read_dir(&themes_dir)
            .unwrap_or_else(|error| panic!("{} 를 읽을 수 없습니다: {error}", themes_dir.display()))
            .flatten()
            .filter(|entry| entry.path().extension().and_then(|extension| extension.to_str()) == Some("json"))
            .filter_map(|entry| {
                entry
                    .path()
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(std::string::ToString::to_string)
            })
            .collect();
        files.sort();

        let mut registered: Vec<String> = BUNDLED_THEME_SOURCES.iter().map(|(id, _)| (*id).to_string()).collect();
        registered.sort();

        assert!(!files.is_empty(), "{} 에 번들 테마 JSON 이 없습니다", themes_dir.display());
        assert_eq!(
            registered, files,
            "BUNDLED_THEME_SOURCES 등록 목록이 resources/themes 의 JSON 파일과 다릅니다"
        );

        let mismatched: Vec<String> = BUNDLED_THEME_SOURCES
            .iter()
            .filter_map(|(id, source)| {
                let theme = serde_json::from_str::<Theme>(source).ok()?;
                (theme.id != *id).then(|| format!("'{id}' 로 등록된 파일의 theme.id 는 '{}' 입니다", theme.id))
            })
            .collect();

        assert!(
            mismatched.is_empty(),
            "번들 테마 등록 id 와 파일 내용이 어긋납니다:\n{}",
            mismatched.join("\n")
        );
    }
}
