use taide_model::theme::ThemeType;

pub const COLOR_NAMESPACES: &[(&str, &[&str])] = &[
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

pub const SYNTAX_SCOPE_CANDIDATES: &[(&str, &[&str])] = &[
    ("keyword", &["keyword.control", "keyword"]),
    ("storage", &["storage.type", "storage.modifier", "storage"]),
    ("operator", &["keyword.operator", "punctuation.separator"]),
    ("string", &["string.quoted", "string"]),
    ("number", &["constant.numeric"]),
    ("regexp", &["string.regexp"]),
    ("comment", &["comment.line", "comment"]),
    ("docComment", &["comment.block.documentation", "comment"]),
    ("function", &["entity.name.function", "support.function"]),
    (
        "method",
        &["entity.name.function.member", "meta.function-call", "entity.name.function"],
    ),
    ("variable", &["variable.other.readwrite", "variable.other", "variable"]),
    ("parameter", &["variable.parameter"]),
    (
        "property",
        &["variable.other.property", "support.type.property-name", "meta.object-literal.key"],
    ),
    ("type", &["support.type", "entity.name.type"]),
    (
        "class",
        &["entity.name.type.class", "entity.name.class", "support.class", "entity.name.type"],
    ),
    ("interface", &["entity.name.type.interface", "entity.name.type"]),
    ("enum", &["entity.name.type.enum", "entity.name.type"]),
    ("constant", &["constant.language", "variable.other.constant", "constant"]),
    ("namespace", &["entity.name.namespace", "entity.name.type.module"]),
    (
        "decorator",
        &["meta.decorator", "entity.name.function.decorator", "punctuation.decorator"],
    ),
    ("tag", &["entity.name.tag"]),
    ("attribute", &["entity.other.attribute-name"]),
    ("punctuation", &["punctuation.definition", "punctuation"]),
    ("invalid", &["invalid.illegal", "invalid"]),
    ("link", &["markup.underline.link", "string.other.link"]),
    ("markdownHeading", &["markup.heading"]),
    ("markdownEmphasis", &["markup.italic"]),
    ("markdownStrong", &["markup.bold"]),
    ("markdownCode", &["markup.inline.raw", "markup.raw"]),
    ("markdownQuote", &["markup.quote"]),
    ("markdownListMarker", &["punctuation.definition.list", "markup.list"]),
];

pub const TERMINAL_ANSI_TOKENS: [&str; 16] = [
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

pub const TERMINAL_MIRRORED_TOKENS: [(&str, &str); 4] = [
    ("background", "terminal.background"),
    ("foreground", "terminal.foreground"),
    ("cursor", "terminal.cursor"),
    ("selection", "terminal.selection"),
];

pub fn default_ansi_palette(theme_type: ThemeType) -> [&'static str; 16] {
    match theme_type {
        ThemeType::Dark => [
            "#000000", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5", "#666666", "#f14c4c", "#23d18b",
            "#f5f543", "#3b8eea", "#d670d6", "#29b8db", "#e5e5e5",
        ],
        ThemeType::Light => [
            "#000000", "#cd3131", "#107c10", "#949800", "#0451a5", "#bc05bc", "#0598bc", "#555555", "#666666", "#cd3131", "#14ce14",
            "#b5ba00", "#0451a5", "#bc05bc", "#0598bc", "#a5a5a5",
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Foreground,
    Background,
    Border,
    Status,
    Shadow,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Foreground => "foreground",
            Self::Background => "background",
            Self::Border => "border",
            Self::Status => "status",
            Self::Shadow => "shadow",
        }
    }

    pub fn family_fallback_keys(self) -> &'static [&'static str] {
        match self {
            Self::Foreground => &["editor.foreground", "foreground"],
            Self::Background => &["editor.background"],
            Self::Border => &["panel.border", "editorGroup.border", "contrastBorder"],
            Self::Status | Self::Shadow => &[],
        }
    }

    pub fn safe_default(self, theme_type: ThemeType) -> &'static str {
        match (theme_type, self) {
            (ThemeType::Dark, Self::Foreground) => "#D4D4D4",
            (ThemeType::Dark, Self::Background) => "#1E1E1E",
            (ThemeType::Dark, Self::Border) => "#3C3C3C",
            (ThemeType::Dark, Self::Status) => "#569CD6",
            (ThemeType::Dark, Self::Shadow) => "#00000080",
            (ThemeType::Light, Self::Foreground) => "#1E1E1E",
            (ThemeType::Light, Self::Background) => "#FFFFFF",
            (ThemeType::Light, Self::Border) => "#D4D4D4",
            (ThemeType::Light, Self::Status) => "#0066BF",
            (ThemeType::Light, Self::Shadow) => "#00000026",
        }
    }
}

pub fn list_hover_default(theme_type: ThemeType) -> &'static str {
    match theme_type {
        ThemeType::Dark => "#2A2D2E",
        ThemeType::Light => "#F0F0F0",
    }
}

pub fn list_active_selection_default(theme_type: ThemeType) -> &'static str {
    match theme_type {
        ThemeType::Dark => "#04395E",
        ThemeType::Light => "#ADD6FF",
    }
}

pub fn list_inactive_selection_default(theme_type: ThemeType) -> &'static str {
    match theme_type {
        ThemeType::Dark => "#37373D",
        ThemeType::Light => "#E4E6F1",
    }
}

pub const MATCH_HIGHLIGHT_FOREGROUND_CANDIDATES: [&str; 2] = ["list.highlightForeground", "editor.findMatchHighlightBackground"];

/// How one TAIDE color token is resolved from a VS Code theme.
#[derive(Clone, Copy, Debug)]
pub enum Rule {
    Chain(&'static [&'static str]),
    Ansi(&'static str),
    Transparent,
    ListHover(&'static str),
    ListSelection(&'static str),
    ExplorerFocused,
    ListActive,
    MatchHighlight,
    RefBranch,
}

pub const COLOR_MAPPING: &[(&str, Category, Rule)] = &[
    ("app.background", Category::Background, Rule::Chain(&["editor.background"])),
    ("app.foreground", Category::Foreground, Rule::Chain(&["foreground"])),
    (
        "app.border",
        Category::Border,
        Rule::Chain(&["panel.border", "editorGroup.border", "contrastBorder"]),
    ),
    ("app.focusBorder", Category::Border, Rule::Chain(&["focusBorder"])),
    ("app.shadow", Category::Shadow, Rule::Chain(&["widget.shadow", "scrollbar.shadow"])),
    (
        "app.accent",
        Category::Status,
        Rule::Chain(&["textLink.foreground", "button.background", "focusBorder"]),
    ),
    (
        "appSidebar.background",
        Category::Background,
        Rule::Chain(&["activityBar.background", "sideBar.background"]),
    ),
    ("appSidebar.itemHover", Category::Background, Rule::Chain(&["list.hoverBackground"])),
    (
        "appSidebar.itemActive",
        Category::Background,
        Rule::Chain(&["list.activeSelectionBackground"]),
    ),
    (
        "appSidebar.iconDefault",
        Category::Foreground,
        Rule::Chain(&["activityBar.inactiveForeground", "icon.foreground"]),
    ),
    ("appSidebar.iconAgentRunning", Category::Status, Rule::Ansi("green")),
    ("appSidebar.iconAgentWorking", Category::Status, Rule::Ansi("blue")),
    ("appSidebar.iconAgentAwaiting", Category::Status, Rule::Ansi("yellow")),
    ("appSidebar.iconAgentIdle", Category::Status, Rule::Ansi("brightGreen")),
    (
        "appSidebar.iconAgentUnknown",
        Category::Foreground,
        Rule::Chain(&["disabledForeground", "descriptionForeground"]),
    ),
    (
        "appSidebar.badge",
        Category::Status,
        Rule::Chain(&["activityBarBadge.background", "badge.background"]),
    ),
    (
        "tabBar.background",
        Category::Background,
        Rule::Chain(&["editorGroupHeader.tabsBackground"]),
    ),
    (
        "tabBar.tabActiveBackground",
        Category::Background,
        Rule::Chain(&["tab.activeBackground"]),
    ),
    (
        "tabBar.tabInactiveBackground",
        Category::Background,
        Rule::Chain(&["tab.inactiveBackground"]),
    ),
    (
        "tabBar.tabActiveForeground",
        Category::Foreground,
        Rule::Chain(&["tab.activeForeground"]),
    ),
    (
        "tabBar.tabInactiveForeground",
        Category::Foreground,
        Rule::Chain(&["tab.inactiveForeground"]),
    ),
    ("tabBar.tabBorder", Category::Border, Rule::Chain(&["tab.border"])),
    (
        "tabBar.tabActiveIndicator",
        Category::Status,
        Rule::Chain(&["tab.activeBorderTop", "tab.activeBorder", "focusBorder"]),
    ),
    (
        "tabBar.dirtyDot",
        Category::Status,
        Rule::Chain(&[
            "tab.activeModifiedBorder",
            "gitDecoration.modifiedResourceForeground",
            "terminal.ansiYellow",
        ]),
    ),
    (
        "tabBar.previewForeground",
        Category::Foreground,
        Rule::Chain(&["tab.unfocusedActiveForeground", "tab.inactiveForeground"]),
    ),
    (
        "tabBar.dropTarget",
        Category::Status,
        Rule::Chain(&["list.dropBackground", "editorGroup.dropBackground", "focusBorder"]),
    ),
    ("explorer.background", Category::Background, Rule::Chain(&["sideBar.background"])),
    ("explorer.itemHover", Category::Background, Rule::ListHover("explorer.background")),
    (
        "explorer.itemSelected",
        Category::Background,
        Rule::ListSelection("explorer.background"),
    ),
    ("explorer.itemFocused", Category::Background, Rule::ExplorerFocused),
    ("explorer.indentGuide", Category::Border, Rule::Chain(&["tree.indentGuidesStroke"])),
    (
        "explorer.folderIcon",
        Category::Foreground,
        Rule::Chain(&["icon.foreground", "sideBar.foreground"]),
    ),
    (
        "explorer.gitModified",
        Category::Status,
        Rule::Chain(&["gitDecoration.modifiedResourceForeground", "terminal.ansiYellow"]),
    ),
    (
        "explorer.gitAdded",
        Category::Status,
        Rule::Chain(&["gitDecoration.addedResourceForeground", "terminal.ansiGreen"]),
    ),
    (
        "explorer.gitDeleted",
        Category::Status,
        Rule::Chain(&["gitDecoration.deletedResourceForeground", "terminal.ansiRed"]),
    ),
    (
        "explorer.gitUntracked",
        Category::Status,
        Rule::Chain(&["gitDecoration.untrackedResourceForeground", "terminal.ansiGreen"]),
    ),
    (
        "explorer.gitIgnored",
        Category::Status,
        Rule::Chain(&["gitDecoration.ignoredResourceForeground", "terminal.ansiBrightBlack"]),
    ),
    ("panel.background", Category::Background, Rule::Chain(&["sideBar.background"])),
    (
        "panel.sectionHeader",
        Category::Foreground,
        Rule::Chain(&["sideBarSectionHeader.foreground", "sideBarTitle.foreground"]),
    ),
    ("panel.inputBackground", Category::Background, Rule::Chain(&["input.background"])),
    (
        "panel.inputBorder",
        Category::Border,
        Rule::Chain(&["input.border", "panelInput.border", "dropdown.border"]),
    ),
    ("panel.matchHighlight", Category::Status, Rule::MatchHighlight),
    ("editor.background", Category::Background, Rule::Chain(&["editor.background"])),
    ("editor.foreground", Category::Foreground, Rule::Chain(&["editor.foreground"])),
    (
        "editor.lineHighlight",
        Category::Background,
        Rule::Chain(&["editor.lineHighlightBackground"]),
    ),
    (
        "editor.cursor",
        Category::Status,
        Rule::Chain(&["editorCursor.foreground", "terminal.ansiWhite"]),
    ),
    (
        "editor.selection",
        Category::Background,
        Rule::Chain(&["editor.selectionBackground"]),
    ),
    (
        "editor.inactiveSelection",
        Category::Background,
        Rule::Chain(&["editor.inactiveSelectionBackground"]),
    ),
    (
        "editor.lineNumber",
        Category::Foreground,
        Rule::Chain(&["editorLineNumber.foreground"]),
    ),
    (
        "editor.lineNumberActive",
        Category::Foreground,
        Rule::Chain(&["editorLineNumber.activeForeground"]),
    ),
    (
        "editor.indentGuide",
        Category::Border,
        Rule::Chain(&["editorIndentGuide.background1", "editorIndentGuide.background"]),
    ),
    (
        "editor.whitespace",
        Category::Foreground,
        Rule::Chain(&["editorWhitespace.foreground"]),
    ),
    (
        "editor.bracketMatch",
        Category::Border,
        Rule::Chain(&["editorBracketMatch.border", "editorBracketMatch.background"]),
    ),
    (
        "editor.findMatch",
        Category::Status,
        Rule::Chain(&["editor.findMatchBackground", "terminal.ansiYellow"]),
    ),
    (
        "editor.findMatchHighlight",
        Category::Status,
        Rule::Chain(&["editor.findMatchHighlightBackground", "@editor.findMatch"]),
    ),
    (
        "editor.hoverBackground",
        Category::Background,
        Rule::Chain(&["editorHoverWidget.background"]),
    ),
    (
        "editor.widgetBackground",
        Category::Background,
        Rule::Chain(&["editorWidget.background", "editorSuggestWidget.background"]),
    ),
    (
        "editor.widgetBorder",
        Category::Border,
        Rule::Chain(&["editorWidget.border", "editorHoverWidget.border"]),
    ),
    (
        "editorGutter.addedBackground",
        Category::Status,
        Rule::Chain(&["editorGutter.addedBackground", "terminal.ansiGreen"]),
    ),
    (
        "editorGutter.modifiedBackground",
        Category::Status,
        Rule::Chain(&["editorGutter.modifiedBackground", "terminal.ansiBlue"]),
    ),
    (
        "editorGutter.deletedBackground",
        Category::Status,
        Rule::Chain(&["editorGutter.deletedBackground", "terminal.ansiRed"]),
    ),
    (
        "editorBlame.foreground",
        Category::Foreground,
        Rule::Chain(&["editorCodeLens.foreground", "editorInlayHint.foreground", "descriptionForeground"]),
    ),
    ("editorBlame.background", Category::Background, Rule::Transparent),
    (
        "diff.insertedBackground",
        Category::Status,
        Rule::Chain(&["diffEditor.insertedTextBackground", "terminal.ansiGreen"]),
    ),
    (
        "diff.insertedLineBackground",
        Category::Status,
        Rule::Chain(&["diffEditor.insertedLineBackground", "@diff.insertedBackground"]),
    ),
    (
        "diff.removedBackground",
        Category::Status,
        Rule::Chain(&["diffEditor.removedTextBackground", "terminal.ansiRed"]),
    ),
    (
        "diff.removedLineBackground",
        Category::Status,
        Rule::Chain(&["diffEditor.removedLineBackground", "@diff.removedBackground"]),
    ),
    (
        "diff.border",
        Category::Border,
        Rule::Chain(&["diffEditor.border", "editorGroup.border"]),
    ),
    (
        "terminal.background",
        Category::Background,
        Rule::Chain(&["terminal.background", "panel.background", "editor.background"]),
    ),
    (
        "terminal.foreground",
        Category::Foreground,
        Rule::Chain(&["terminal.foreground", "foreground"]),
    ),
    (
        "terminal.cursor",
        Category::Status,
        Rule::Chain(&["terminalCursor.foreground", "editorCursor.foreground", "terminal.ansiWhite"]),
    ),
    (
        "terminal.selection",
        Category::Background,
        Rule::Chain(&["terminal.selectionBackground"]),
    ),
    (
        "terminal.commandBlockBorder",
        Category::Border,
        Rule::Chain(&["panel.border", "@app.border"]),
    ),
    (
        "terminal.linkForeground",
        Category::Foreground,
        Rule::Chain(&["textLink.foreground", "editorLink.activeForeground"]),
    ),
    (
        "git.added",
        Category::Status,
        Rule::Chain(&["gitDecoration.addedResourceForeground", "terminal.ansiGreen"]),
    ),
    (
        "git.modified",
        Category::Status,
        Rule::Chain(&["gitDecoration.modifiedResourceForeground", "terminal.ansiYellow"]),
    ),
    (
        "git.deleted",
        Category::Status,
        Rule::Chain(&["gitDecoration.deletedResourceForeground", "terminal.ansiRed"]),
    ),
    (
        "git.renamed",
        Category::Status,
        Rule::Chain(&["gitDecoration.renamedResourceForeground", "terminal.ansiBlue"]),
    ),
    (
        "git.untracked",
        Category::Status,
        Rule::Chain(&["gitDecoration.untrackedResourceForeground", "terminal.ansiGreen"]),
    ),
    (
        "git.conflicted",
        Category::Status,
        Rule::Chain(&["gitDecoration.conflictingResourceForeground", "terminal.ansiMagenta"]),
    ),
    (
        "git.staged",
        Category::Status,
        Rule::Chain(&["gitDecoration.stageModifiedResourceForeground", "@git.modified"]),
    ),
    ("graph.lane1", Category::Status, Rule::Ansi("blue")),
    ("graph.lane2", Category::Status, Rule::Ansi("green")),
    ("graph.lane3", Category::Status, Rule::Ansi("yellow")),
    ("graph.lane4", Category::Status, Rule::Ansi("magenta")),
    ("graph.lane5", Category::Status, Rule::Ansi("cyan")),
    ("graph.lane6", Category::Status, Rule::Ansi("red")),
    ("graph.lane7", Category::Status, Rule::Ansi("brightBlue")),
    ("graph.lane8", Category::Status, Rule::Ansi("brightGreen")),
    ("graph.lane9", Category::Status, Rule::Ansi("brightYellow")),
    ("graph.lane10", Category::Status, Rule::Ansi("brightMagenta")),
    ("graph.lane11", Category::Status, Rule::Ansi("brightCyan")),
    ("graph.lane12", Category::Status, Rule::Ansi("brightRed")),
    ("graph.refBranch", Category::Status, Rule::RefBranch),
    ("graph.refTag", Category::Status, Rule::Ansi("yellow")),
    ("graph.refHead", Category::Status, Rule::Ansi("green")),
    (
        "statusIndicator.info",
        Category::Status,
        Rule::Chain(&["editorInfo.foreground", "notificationsInfoIcon.foreground", "terminal.ansiBlue"]),
    ),
    (
        "statusIndicator.warning",
        Category::Status,
        Rule::Chain(&[
            "editorWarning.foreground",
            "notificationsWarningIcon.foreground",
            "terminal.ansiYellow",
        ]),
    ),
    (
        "statusIndicator.error",
        Category::Status,
        Rule::Chain(&["editorError.foreground", "errorForeground", "terminal.ansiRed"]),
    ),
    ("statusIndicator.success", Category::Status, Rule::Ansi("green")),
    (
        "menu.background",
        Category::Background,
        Rule::Chain(&["menu.background", "dropdown.background"]),
    ),
    ("menu.border", Category::Border, Rule::Chain(&["menu.border", "dropdown.border"])),
    (
        "menu.itemHover",
        Category::Background,
        Rule::Chain(&["list.hoverBackground", "menu.selectionBackground"]),
    ),
    ("menu.separator", Category::Border, Rule::Chain(&["menu.separatorBackground"])),
    (
        "popover.background",
        Category::Background,
        Rule::Chain(&["editorWidget.background", "menu.background"]),
    ),
    ("popover.border", Category::Border, Rule::Chain(&["editorWidget.border"])),
    ("popover.itemHover", Category::Background, Rule::Chain(&["list.hoverBackground"])),
    ("popover.separator", Category::Border, Rule::Chain(&["menu.separatorBackground"])),
    (
        "tooltip.background",
        Category::Background,
        Rule::Chain(&["editorHoverWidget.background", "editorWidget.background"]),
    ),
    ("tooltip.border", Category::Border, Rule::Chain(&["editorHoverWidget.border"])),
    ("tooltip.itemHover", Category::Background, Rule::Chain(&["list.hoverBackground"])),
    ("tooltip.separator", Category::Border, Rule::Chain(&["menu.separatorBackground"])),
    (
        "modal.background",
        Category::Background,
        Rule::Chain(&["editorWidget.background", "notifications.background"]),
    ),
    (
        "modal.border",
        Category::Border,
        Rule::Chain(&["editorWidget.border", "notificationCenter.border"]),
    ),
    ("modal.itemHover", Category::Background, Rule::Chain(&["list.hoverBackground"])),
    ("modal.separator", Category::Border, Rule::Chain(&["menu.separatorBackground"])),
    (
        "scrollbar.thumb",
        Category::Background,
        Rule::Chain(&["scrollbarSlider.background"]),
    ),
    (
        "scrollbar.thumbHover",
        Category::Background,
        Rule::Chain(&["scrollbarSlider.hoverBackground"]),
    ),
    ("scrollbar.track", Category::Background, Rule::Transparent),
    ("input.background", Category::Background, Rule::Chain(&["input.background"])),
    ("input.foreground", Category::Foreground, Rule::Chain(&["input.foreground"])),
    ("input.border", Category::Border, Rule::Chain(&["input.border", "dropdown.border"])),
    (
        "input.placeholder",
        Category::Foreground,
        Rule::Chain(&["input.placeholderForeground"]),
    ),
    ("input.focusBorder", Category::Border, Rule::Chain(&["focusBorder"])),
    (
        "button.background",
        Category::Background,
        Rule::Chain(&["button.secondaryBackground", "button.background"]),
    ),
    (
        "button.foreground",
        Category::Foreground,
        Rule::Chain(&["button.secondaryForeground", "button.foreground"]),
    ),
    (
        "button.hoverBackground",
        Category::Background,
        Rule::Chain(&["button.secondaryHoverBackground", "button.hoverBackground"]),
    ),
    (
        "button.primaryBackground",
        Category::Background,
        Rule::Chain(&["button.background"]),
    ),
    (
        "button.primaryForeground",
        Category::Foreground,
        Rule::Chain(&["button.foreground"]),
    ),
    ("list.background", Category::Background, Rule::Chain(&["sideBar.background"])),
    ("list.hoverBackground", Category::Background, Rule::ListHover("list.background")),
    ("list.activeBackground", Category::Background, Rule::ListActive),
    (
        "list.foreground",
        Category::Foreground,
        Rule::Chain(&["sideBar.foreground", "foreground"]),
    ),
];

/// A foreground that must stay readable on a background.
pub struct ContrastPair {
    pub label: &'static str,
    pub foreground: &'static str,
    pub background: &'static str,
    pub is_blocking: bool,
}

pub const CONTRAST_PAIRS: [ContrastPair; 7] = [
    ContrastPair {
        label: "app",
        foreground: "app.foreground",
        background: "app.background",
        is_blocking: true,
    },
    ContrastPair {
        label: "editor",
        foreground: "editor.foreground",
        background: "editor.background",
        is_blocking: true,
    },
    ContrastPair {
        label: "panel",
        foreground: "panel.sectionHeader",
        background: "panel.background",
        is_blocking: true,
    },
    ContrastPair {
        label: "tooltip",
        foreground: "app.foreground",
        background: "tooltip.background",
        is_blocking: true,
    },
    ContrastPair {
        label: "matchHighlight",
        foreground: "panel.matchHighlight",
        background: "panel.background",
        is_blocking: true,
    },
    ContrastPair {
        label: "selectionMatchHighlight",
        foreground: "panel.matchHighlight",
        background: "list.activeBackground",
        is_blocking: false,
    },
    ContrastPair {
        label: "selectionForeground",
        foreground: "list.foreground",
        background: "list.activeBackground",
        is_blocking: false,
    },
];

/// A foreground drawn over a background that itself sits on a surface.
pub struct ComponentPair {
    pub label: &'static str,
    pub foreground: &'static str,
    pub background: &'static str,
    pub surface: &'static str,
}

const fn component(label: &'static str, foreground: &'static str, background: &'static str, surface: &'static str) -> ComponentPair {
    ComponentPair {
        label,
        foreground,
        background,
        surface,
    }
}

pub const COMPONENT_CONTRAST_PAIRS: &[ComponentPair] = &[
    component("listRow", "list.foreground", "list.background", "app.background"),
    component("listHoverRow", "list.foreground", "list.hoverBackground", "list.background"),
    component("explorerGitAdded", "explorer.gitAdded", "explorer.background", "app.background"),
    component(
        "explorerGitModified",
        "explorer.gitModified",
        "explorer.background",
        "app.background",
    ),
    component("explorerGitDeleted", "explorer.gitDeleted", "explorer.background", "app.background"),
    component(
        "explorerGitUntracked",
        "explorer.gitUntracked",
        "explorer.background",
        "app.background",
    ),
    component("gitAdded", "git.added", "explorer.background", "app.background"),
    component("gitModified", "git.modified", "explorer.background", "app.background"),
    component("gitDeleted", "git.deleted", "explorer.background", "app.background"),
    component("gitRenamed", "git.renamed", "explorer.background", "app.background"),
    component("gitUntracked", "git.untracked", "explorer.background", "app.background"),
    component("gitConflicted", "git.conflicted", "explorer.background", "app.background"),
    component("gitStaged", "git.staged", "explorer.background", "app.background"),
    component("statusBarText", "appSidebar.iconDefault", "appSidebar.background", "app.background"),
    component("panelMutedText", "appSidebar.iconDefault", "panel.background", "app.background"),
    component(
        "explorerMutedText",
        "appSidebar.iconDefault",
        "explorer.background",
        "app.background",
    ),
    component("panelBadge", "appSidebar.badge", "panel.background", "app.background"),
    component("statusBarError", "statusIndicator.error", "appSidebar.background", "app.background"),
    component(
        "statusBarWarning",
        "statusIndicator.warning",
        "appSidebar.background",
        "app.background",
    ),
    component(
        "statusBarSuccess",
        "statusIndicator.success",
        "appSidebar.background",
        "app.background",
    ),
    component("problemError", "statusIndicator.error", "panel.background", "app.background"),
    component("problemWarning", "statusIndicator.warning", "panel.background", "app.background"),
    component("problemInfo", "statusIndicator.info", "panel.background", "app.background"),
    component("problemSuccess", "statusIndicator.success", "panel.background", "app.background"),
    component("inputText", "input.foreground", "input.background", "editor.widgetBackground"),
    component(
        "inputPlaceholder",
        "input.placeholder",
        "input.background",
        "editor.widgetBackground",
    ),
    component(
        "buttonPrimary",
        "button.primaryForeground",
        "button.primaryBackground",
        "panel.background",
    ),
    component("buttonSecondary", "button.foreground", "button.background", "panel.background"),
    component(
        "buttonSecondaryHover",
        "button.foreground",
        "button.hoverBackground",
        "panel.background",
    ),
    component(
        "tabActive",
        "tabBar.tabActiveForeground",
        "tabBar.tabActiveBackground",
        "app.background",
    ),
    component(
        "tabInactive",
        "tabBar.tabInactiveForeground",
        "tabBar.tabInactiveBackground",
        "app.background",
    ),
    component(
        "tabPreview",
        "tabBar.previewForeground",
        "tabBar.tabActiveBackground",
        "app.background",
    ),
    component("terminalText", "terminal.foreground", "terminal.background", "app.background"),
    component("editorLink", "terminal.linkForeground", "editor.background", "app.background"),
    component(
        "editorWidgetLink",
        "terminal.linkForeground",
        "editor.widgetBackground",
        "app.background",
    ),
    component("menuItemHoverText", "app.foreground", "menu.itemHover", "menu.background"),
];

pub const STATE_MIN_DISTINCT_DELTA_E: f64 = 2.3;
pub const SUBTLE_STATE_MIN_DISTINCT_DELTA_E: f64 = 1.0;
pub const APP_SHADOW_MIN_ALPHA: f64 = 0.149;

/// A state color that must stay distinguishable from the container it renders on.
pub struct StatePair {
    pub label: &'static str,
    pub state: &'static str,
    pub container: &'static str,
    pub min_delta_e: f64,
    pub surface: Option<&'static str>,
    pub alternative_state: Option<&'static str>,
    pub is_sibling: bool,
}

const fn state(label: &'static str, state: &'static str, container: &'static str) -> StatePair {
    StatePair {
        label,
        state,
        container,
        min_delta_e: STATE_MIN_DISTINCT_DELTA_E,
        surface: None,
        alternative_state: None,
        is_sibling: false,
    }
}

const fn subtle(label: &'static str, state_key: &'static str, container: &'static str) -> StatePair {
    StatePair {
        min_delta_e: SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
        ..state(label, state_key, container)
    }
}

const fn on(label: &'static str, state_key: &'static str, container: &'static str, surface: &'static str) -> StatePair {
    StatePair {
        surface: Some(surface),
        ..state(label, state_key, container)
    }
}

const fn sibling(label: &'static str, state_key: &'static str, container: &'static str, surface: &'static str) -> StatePair {
    StatePair {
        is_sibling: true,
        ..on(label, state_key, container, surface)
    }
}

pub const STATE_DISTINCTNESS_PAIRS: &[StatePair] = &[
    state("editorSelection", "editor.selection", "editor.background"),
    subtle("editorInactiveSelection", "editor.inactiveSelection", "editor.background"),
    subtle("editorCurrentLine", "editor.lineHighlight", "editor.background"),
    state("editorFindMatch", "editor.findMatch", "editor.background"),
    subtle("editorFindMatchHighlight", "editor.findMatchHighlight", "editor.background"),
    state("editorBracketMatch", "editor.bracketMatch", "editor.background"),
    state("editorCursor", "editor.cursor", "editor.background"),
    state("terminalSelection", "terminal.selection", "terminal.background"),
    state("terminalCursor", "terminal.cursor", "terminal.background"),
    state("listHover", "list.hoverBackground", "list.background"),
    state("listActive", "list.activeBackground", "list.background"),
    sibling(
        "listActiveVsHover",
        "list.activeBackground",
        "list.hoverBackground",
        "list.background",
    ),
    on(
        "listHoverOnActiveTab",
        "list.hoverBackground",
        "tabBar.tabActiveBackground",
        "app.background",
    ),
    on(
        "listHoverOnInactiveTab",
        "list.hoverBackground",
        "tabBar.tabInactiveBackground",
        "app.background",
    ),
    state("explorerItemHover", "explorer.itemHover", "explorer.background"),
    state("explorerItemSelected", "explorer.itemSelected", "explorer.background"),
    state("explorerItemFocused", "explorer.itemFocused", "explorer.background"),
    sibling(
        "explorerSelectedVsHover",
        "explorer.itemSelected",
        "explorer.itemHover",
        "explorer.background",
    ),
    state("explorerHoverOnPanel", "explorer.itemHover", "panel.background"),
    state("explorerSelectedOnPanel", "explorer.itemSelected", "panel.background"),
    state("explorerFocusedOnPanel", "explorer.itemFocused", "panel.background"),
    on(
        "explorerHoverOnStatusBar",
        "explorer.itemHover",
        "appSidebar.background",
        "app.background",
    ),
    on(
        "explorerSelectedOnStatusBar",
        "explorer.itemSelected",
        "appSidebar.background",
        "app.background",
    ),
    state("explorerHoverOnEditorWidget", "explorer.itemHover", "editor.widgetBackground"),
    on(
        "sidebarItemHover",
        "appSidebar.itemHover",
        "appSidebar.background",
        "app.background",
    ),
    on(
        "sidebarItemActive",
        "appSidebar.itemActive",
        "appSidebar.background",
        "app.background",
    ),
    sibling(
        "sidebarActiveVsHover",
        "appSidebar.itemActive",
        "appSidebar.itemHover",
        "appSidebar.background",
    ),
    state("sidebarActiveOnCard", "appSidebar.itemActive", "panel.background"),
    state("sidebarHoverOnCard", "appSidebar.itemHover", "panel.background"),
    state("sidebarHoverOnEditor", "appSidebar.itemHover", "editor.background"),
    state("sidebarHoverOnApp", "appSidebar.itemHover", "app.background"),
    state("sidebarActiveOnApp", "appSidebar.itemActive", "app.background"),
    on(
        "sidebarHoverOnTabBar",
        "appSidebar.itemHover",
        "tabBar.background",
        "app.background",
    ),
    on("sidebarBadge", "appSidebar.badge", "appSidebar.background", "app.background"),
    StatePair {
        alternative_state: Some("tabBar.tabActiveIndicator"),
        ..on("tabActive", "tabBar.tabActiveBackground", "tabBar.background", "app.background")
    },
    StatePair {
        alternative_state: Some("tabBar.tabActiveIndicator"),
        ..sibling(
            "tabActiveVsInactive",
            "tabBar.tabActiveBackground",
            "tabBar.tabInactiveBackground",
            "app.background",
        )
    },
    state("switchCheckedTrackOnCard", "button.primaryBackground", "panel.background"),
    state("switchCheckedTrackVsThumb", "button.primaryBackground", "app.background"),
    state("switchUncheckedTrackOnCard", "input.border", "panel.background"),
    state("switchUncheckedTrackVsThumb", "input.border", "app.background"),
    sibling(
        "switchCheckedVsUncheckedTrack",
        "button.primaryBackground",
        "input.border",
        "panel.background",
    ),
    on("buttonHover", "button.hoverBackground", "button.background", "panel.background"),
    on("inputBorder", "input.border", "input.background", "panel.background"),
    on("panelInputBorder", "panel.inputBorder", "panel.inputBackground", "panel.background"),
    state("focusBorderOnApp", "app.focusBorder", "app.background"),
    state("focusBorderOnCard", "app.focusBorder", "panel.background"),
    on("inputFocusBorder", "input.focusBorder", "input.background", "panel.background"),
    state("menuItemHover", "menu.itemHover", "menu.background"),
    state("modalItemHover", "modal.itemHover", "modal.background"),
    state("scrollbarThumb", "scrollbar.thumb", "scrollbar.track"),
];
