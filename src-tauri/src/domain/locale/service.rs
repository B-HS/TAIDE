use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::domain::locale::types::{LocalePack, LocaleSummary, ResolvedLocale, LOCALE_SCHEMA_VERSION};
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::paths::AppPaths;

pub const BUILTIN_EN_ID: &str = "en";
pub const BUILTIN_KO_ID: &str = "ko";
pub const BUILTIN_JA_ID: &str = "ja";

const MESSAGE_NAMESPACES: &[(&str, &[&str])] = &[
    ("common", &["cancel", "confirm", "close", "save", "loading", "retry"]),
    (
        "app",
        &[
            "selectProject",
            "noProjectOpen",
            "openProject",
            "openSettingsFile",
            "openFolder",
            "openFolderHint",
            "openProjectFirst",
            "recentItems",
            "reloadWindow",
            "dropToOpen",
        ],
    ),
    (
        "explorer",
        &[
            "title",
            "newFile",
            "newFolder",
            "refresh",
            "collapseAll",
            "entryNamePlaceholder",
            "reveal",
            "copyPath",
            "sidebarSwitchLabel",
            "openToTheSide",
            "openWith",
            "openWithEditor",
            "openWithPreview",
            "openInBrowser",
            "revealInFinder",
            "openInTerminal",
            "findInFolder",
            "selectForCompare",
            "compareWithSelected",
            "cut",
            "copy",
            "paste",
            "copyRelativePath",
            "rename",
            "delete",
            "deleteConfirmTitle",
            "deleteConfirmDescription",
            "pasteConflictSuffix",
            "searchScopeLabel",
            "entryNameDuplicate",
            "entryNameInvalidChar",
            "entryNameReserved",
        ],
    ),
    (
        "search",
        &[
            "title",
            "placeholder",
            "noResults",
            "matchCount",
            "caseSensitive",
            "wholeWord",
            "searching",
            "noMatches",
            "pressEnterHint",
            "replaceToggle",
            "replacePlaceholder",
            "replaceAll",
            "replaceConfirmTitle",
            "replaceConfirmDescription",
            "selectFile",
            "replaceDone",
            "regex",
            "clearScope",
            "respectGitignore",
            "recentSearches",
            "excludeGlobPlaceholder",
        ],
    ),
    ("searchEditor", &["title", "contextLinesLabel", "noResults"]),
    (
        "git",
        &[
            "title",
            "stage",
            "unstage",
            "discard",
            "commit",
            "noChanges",
            "notARepository",
            "noRepositoryLabel",
            "initRepository",
            "initSuccess",
            "initFailed",
            "sync",
            "branchFilterPlaceholder",
            "branchNotFound",
            "branchCreateNamed",
            "branchLocal",
            "branchRemote",
            "branchSwitch",
            "branchSwitched",
            "mergeChanges",
            "stagedChanges",
            "changes",
            "unstageAll",
            "stageAll",
            "openFile",
            "openChanges",
            "unstageChanges",
            "stageChanges",
            "graph",
            "stash",
            "stashPush",
            "discardHunkTitle",
            "discardHunkDescription",
            "stashApply",
            "stashDrop",
            "stashEmpty",
            "stashPushed",
            "discardTitle",
            "discardDescription",
            "discardConfirm",
            "stageAllTitle",
            "stageAllDescription",
            "commitMessagePlaceholder",
            "committing",
            "generateCommitMessage",
            "generatingCommitMessage",
            "generateCommitMessageFailed",
            "noChangesForCommitMessage",
            "commitMessageGenerated",
            "commitMessageDiffTruncated",
            "commitMessageFilesSkipped",
            "commitMessageEmptyResponse",
            "timeJustNow",
            "timeDaysAgo",
            "timeHoursAgo",
            "timeMinutesAgo",
            "fileCount",
            "acceptCurrentChange",
            "acceptIncomingChange",
            "acceptBothChanges",
            "conflictResolved",
            "resolveConflictTitle",
            "resolveConflictDescription",
            "compareChanges",
            "changedFiles",
            "fileHistory",
            "noFileHistory",
            "revert",
            "revertSuccess",
            "revertConflict",
            "createTag",
            "deleteTag",
            "tagNamePlaceholder",
            "tagMessagePlaceholder",
            "tagCreated",
            "tagDeleted",
            "checkoutRemoteBranch",
            "toggleBlame",
            "commitMessageUsedUnstaged",
        ],
    ),
    (
        "terminal",
        &[
            "title",
            "processExited",
            "restart",
            "copyImeDebugLog",
            "imeDebugCopied",
            "runSelectedText",
            "openLinkFailed",
        ],
    ),
    ("task", &["runTask", "sourceNpm", "sourceMake", "sourceCargo", "noTasksFound"]),
    (
        "editor",
        &[
            "noFileOpen",
            "keepMine",
            "openFailed",
            "cannotOpen",
            "binaryOrTooLarge",
            "readOnlyLargeFile",
            "changedOnDisk",
            "viewDiskContent",
            "renameUnavailable",
            "diffLoadFailed",
            "toggleMarkdownPreview",
            "toggleMinimap",
            "openExternally",
            "cursorPosition",
            "mirrorRestored",
            "mirrorRestoredConflict",
            "codeActionsOnSaveSkipped",
            "workspaceEditApplyFailed",
        ],
    ),
    (
        "tab",
        &[
            "close",
            "closeOthers",
            "closeAll",
            "closeToRight",
            "closeSaved",
            "pin",
            "unpin",
            "split",
            "copyRelativePath",
            "openChanges",
            "closeAriaLabel",
            "unpinAriaLabel",
            "keepOpen",
            "revealInExplorerView",
            "reopenEditorWith",
            "newUntitledFile",
            "newTerminal",
            "saveAsTitle",
            "newTabMenu",
            "moveToNewWindow",
            "moveToMainWindow",
            "moveToWindowNumbered",
        ],
    ),
    ("editorArea", &["splitLeft", "splitRight", "splitTop", "splitBottom"]),
    (
        "palette",
        &[
            "commandPlaceholder",
            "filePlaceholder",
            "noResults",
            "commands",
            "files",
            "title",
            "symbolPlaceholder",
            "linePlaceholder",
            "workspaceSymbolPlaceholder",
            "symbols",
            "workspaceSymbols",
            "noActiveFile",
        ],
    ),
    (
        "settings",
        &[
            "title",
            "appearance",
            "theme",
            "followSystemTheme",
            "language",
            "editorFontSize",
            "terminalFontSize",
            "shell",
            "lspStatus",
            "plugins",
            "loading",
            "lspDescription",
            "themeDark",
            "themeLight",
            "lspInstalled",
            "lspNotInstalled",
            "lspCopyCommand",
            "lspCommandCopied",
            "lspCommandCopyFailed",
            "lspInstall",
            "lspInstalling",
            "lspInstallFailed",
            "lspCancel",
            "lspToolchainMissing",
            "lspSdkMissing",
            "lspExperimental",
            "lspChecksumPending",
            "pluginsManifestHint",
            "pluginsOpenFolder",
            "pluginsReload",
            "pluginsEmpty",
            "pluginEnabled",
            "pluginDisabled",
            "pluginError.parseFailed",
            "pluginError.idMismatch",
            "pluginError.versionMismatch",
            "pluginError.pathEscape",
            "pluginError.grammarMissing",
            "pluginError.grammarInvalid",
            "pluginError.grammarConflict",
            "editorMinimap",
            "showSystemUsage",
            "languageSelectPlaceholder",
            "localesOpenFolder",
            "fontFamilySelectPlaceholder",
            "agentStatusBadge",
            "agentHooks",
            "agentHooksHint",
            "agentHooksAgentClaude",
            "agentHooksAgentCodex",
            "agentHooksAgentGemini",
            "agentHooksUserLevelDescription",
            "agentHooksUserLevelWarning",
            "agentHooksCliMissing",
            "cliInstallButton",
            "cliUninstallButton",
            "cliInstallSuccess",
            "cliUninstallSuccess",
            "cliInstallFailed",
            "cliUninstallFailed",
            "cliStatusInstalled",
            "cliStatusDangling",
            "ideIntegration",
            "ideIntegrationHint",
            "ideAutoOpenDiff",
            "editorWordWrap",
            "editorLineNumbers",
            "editorTabSize",
            "editorInsertSpaces",
            "editorDetectIndentation",
            "editorDetectIndentationHint",
            "editorRenderWhitespace",
            "editorBracketPairColorization",
            "editorFontLigatures",
            "editorCursorStyle",
            "editorCursorBlinking",
            "editorScrollBeyondLastLine",
            "editorStickyScroll",
            "editorStickyScrollDescription",
            "organizeImportsOnSave",
            "organizeImportsOnSaveDescription",
            "fixAllOnSave",
            "fixAllOnSaveDescription",
            "editorCodeLens",
            "editorCodeLensDescription",
            "terminalScrollback",
            "terminalCursorStyle",
            "terminalCursorBlink",
            "enablePreviewTabs",
            "enablePreviewTabsHint",
            "cursorStyleLine",
            "cursorStyleBlock",
            "cursorStyleUnderline",
            "cursorStyleBar",
            "cursorBlinkingBlink",
            "cursorBlinkingSmooth",
            "cursorBlinkingPhase",
            "cursorBlinkingExpand",
            "cursorBlinkingSolid",
            "renderWhitespaceNone",
            "renderWhitespaceBoundary",
            "renderWhitespaceSelection",
            "renderWhitespaceAll",
            "bundledThemesSection",
            "builtinThemesSection",
            "themesOpenFolder",
            "themeImportSuccess",
            "themeImportSaveFailure",
            "themeImportDuplicate",
            "themeImportThemeParseFailure",
            "themeImportThemeIncomplete",
            "themeImportThemeContrastFailure",
            "aiSectionTitle",
            "aiProviderLabel",
            "aiProviderOllamaCloud",
            "aiProviderCodex",
            "aiTokenPlaceholder",
            "aiTokenSave",
            "aiTokenSaved",
            "aiTokenClear",
            "aiTokenSaveFailed",
            "aiTokenNotSet",
            "aiModelLabel",
            "aiModelSelectPlaceholder",
            "aiModelLoadFailed",
            "aiAutoTabToggle",
            "aiAutoTabToggleHint",
            "aiAutoTabProviderRequired",
            "aiCodexUnofficialWarning",
            "syncSectionTitle",
            "syncDescription",
            "syncTokenScopeHint",
            "syncTokenPlaceholder",
            "syncConnect",
            "syncConnectFailed",
            "syncGistIdLabel",
            "syncGistIdNone",
            "syncLastSyncedLabel",
            "syncLastSyncedNever",
            "syncUploadNow",
            "syncUploadFailed",
            "syncUploadSuccess",
            "syncDownloadNow",
            "syncDownloadFailed",
            "syncDownloadSuccess",
            "syncDisconnect",
            "syncDisconnectFailed",
            "syncDisconnected",
            "syncRemoteNewerBadge",
            "syncSecretGistWarning",
            "syncConflictTitle",
            "syncConflictDescription",
            "syncConflictKeepLocal",
            "syncConflictPullRemote",
            "interface",
            "toastPosition",
            "resizerThickness",
            "positionTopLeft",
            "positionTopCenter",
            "positionTopRight",
            "positionMiddleLeft",
            "positionMiddleCenter",
            "positionMiddleRight",
            "positionBottomLeft",
            "positionBottomCenter",
            "positionBottomRight",
            "editor",
            "terminal",
            "systemLanguage",
            "editorFontFamily",
            "terminalFontFamily",
            "fontFamilySystemDefault",
            "fontFamilySearchPlaceholder",
            "fontFamilyMonospaceOnly",
            "fontFamilyNoResults",
            "formatOnSave",
            "autoSaveDelayMs",
            "autoSaveDelayHint",
            "keymap",
            "keymapDescription",
            "keymapChange",
            "keymapReset",
            "keymapCapturePrompt",
            "keymapConflictWarning",
            "keymapOpenEditor",
            "keymapEditorTitle",
            "keymapSearchPlaceholder",
            "keymapSearchByKey",
            "keymapUnassigned",
            "keymapUnbind",
            "keymapResetOne",
            "keymapSourceDefault",
            "keymapSourceUser",
            "keymapConflictBadge",
            "keymapConflictFilter",
            "keymapUnassignedFilter",
            "keymapCommandColumn",
            "keymapKeyColumn",
            "keymapKeyNotBindable",
            "keymapModifierRequired",
            "keymapSourceColumn",
            "keymapNoResults",
            "aiProviderOmlx",
            "aiOmlxBaseUrlLabel",
            "aiOmlxBaseUrlPlaceholder",
            "aiOmlxApiKeyOptional",
            "aiOmlxConnectFailed",
            "editorSemanticHighlighting",
            "editorSemanticHighlightingDescription",
            "editorFormatOnType",
            "editorFormatOnTypeDescription",
            "editorFormatOnPaste",
            "editorFormatOnPasteDescription",
            "emmetEnabled",
            "emmetEnabledDescription",
            "snippetsSectionTitle",
            "snippetsOpenFolder",
            "snippetsManage",
            "keymapChordWaitingBadge",
            "keymapChordCapturePrompt",
            "keymapChordConfirmSingle",
            "keymapInspectorTitle",
            "keymapInspectorEmpty",
            "settingsJsonInvalid",
            "zenFullscreen",
            "zenFullscreenDescription",
            "zenHideStatusBar",
            "zenHideStatusBarDescription",
            "pluginInstallButton",
            "pluginInstallDialogTitle",
            "pluginInstallSuccess",
            "pluginInstallFailed",
            "pluginUninstallButton",
            "pluginUninstallConfirmTitle",
            "pluginUninstallConfirmDescription",
            "pluginUninstallSuccess",
            "pluginUninstallFailed",
            "pluginImportVsixButton",
            "pluginImportVsixDialogTitle",
            "pluginImportVsixThemesSection",
            "pluginImportVsixGrammarsSection",
            "pluginImportVsixNoThemes",
            "pluginImportVsixSuccess",
            "pluginImportVsixFailed",
        ],
    ),
    ("theme", &["loadFailed"]),
    (
        "themeEditor",
        &[
            "backToSettings",
            "createNew",
            "duplicateTheme",
            "editTheme",
            "deleteTheme",
            "deleteConfirmTitle",
            "deleteConfirmDescription",
            "customThemes",
            "noCustomThemes",
            "themeNamePlaceholder",
            "duplicateNameTemplate",
            "save",
            "searchTokensPlaceholder",
            "changedCount",
            "resetToken",
            "invalidColor",
            "colorValuePlaceholder",
            "transparentLabel",
            "pickColor",
            "saturationValueSliderLabel",
            "hueSliderLabel",
            "boldAbbreviation",
            "italicAbbreviation",
            "boldToggle",
            "italicToggle",
            "previewTitle",
            "previewEditorTab",
            "previewTerminalTab",
            "previewTerminalPrompt",
            "previewTerminalCommand",
            "previewCommentText",
            "syntaxSectionTitle",
            "terminalSectionTitle",
            "previewSyntaxTitle",
            "previewAnsiTitle",
            "livePreviewHint",
            "ns.app",
            "ns.appSidebar",
            "ns.tabBar",
            "ns.explorer",
            "ns.panel",
            "ns.editor",
            "ns.editorGutter",
            "ns.editorBlame",
            "ns.diff",
            "ns.terminal",
            "ns.git",
            "ns.graph",
            "ns.statusIndicator",
            "ns.menu",
            "ns.popover",
            "ns.tooltip",
            "ns.modal",
            "ns.scrollbar",
            "ns.input",
            "ns.button",
            "ns.list",
        ],
    ),
    (
        "snippetEditor",
        &[
            "backToSettings",
            "fileListTitle",
            "noFiles",
            "newFileButton",
            "newFileDialogTitle",
            "newFileLanguagePlaceholder",
            "newFileGlobalOption",
            "newFileGlobalNamePlaceholder",
            "deleteFileButton",
            "deleteFileConfirmTitle",
            "deleteFileConfirmDescription",
            "snippetListTitle",
            "noSnippets",
            "addSnippetButton",
            "nameLabel",
            "namePlaceholder",
            "prefixLabel",
            "prefixPlaceholder",
            "prefixHint",
            "bodyLabel",
            "bodyPlaceholder",
            "bodyHint",
            "descriptionLabel",
            "descriptionPlaceholder",
            "scopeLabel",
            "scopePlaceholder",
            "scopeHint",
            "save",
            "saveSuccess",
            "saveFailed",
            "deleteSnippetButton",
            "deleteConfirmTitle",
            "deleteConfirmDescription",
            "parseError",
            "invalidFileName",
            "duplicateNameError",
        ],
    ),
    ("project", &["close", "openInFileManager", "copyPath"]),
    ("sidebar", &["projectsAriaLabel", "openFolderAriaLabel", "settingsAriaLabel"]),
    (
        "window",
        &[
            "titleSeparator",
            "editorFontSize",
            "terminalFontSize",
            "decreaseFontSize",
            "increaseFontSize",
            "resetFontSize",
            "lspStatus",
            "systemUsage",
            "systemUsageHint",
            "systemUsageDetailTitle",
            "systemUsageKindApp",
            "systemUsageKindTerminal",
            "systemUsageKindLsp",
            "systemUsageKindAgent",
            "systemUsageKindOther",
            "systemUsageProcessColumn",
            "systemUsageCpuColumn",
            "systemUsageMemoryColumn",
            "systemUsageEmpty",
        ],
    ),
    (
        "keymap",
        &[
            "quickOpen",
            "closeTab",
            "toggleSidebar",
            "toggleZenMode",
            "find",
            "search",
            "searchReplace",
            "explorer",
            "split",
            "tabCycleNext",
            "tabCyclePrev",
            "reopenClosedTab",
            "save",
            "toggleTerminal",
            "newTerminal",
            "terminalJumpToPreviousCommand",
            "terminalJumpToNextCommand",
            "commandPalette",
            "fontSizeUp",
            "fontSizeDown",
            "category.app",
            "category.view",
            "category.editor",
            "category.tab",
            "category.terminal",
            "category.search",
            "category.file",
            "category.sync",
            "category.window",
            "category.editorSuggest",
            "category.editorNavigation",
            "category.editorSelection",
            "category.editorLines",
            "category.editorFolding",
            "category.editorFormat",
            "category.editorRefactor",
            "category.editorDisplay",
            "category.shellCommand",
            "category.git",
            "cliInstall",
            "cliUninstall",
            "monaco.editor.action.triggerSuggest",
            "monaco.editor.action.triggerParameterHints",
            "monaco.editor.action.showHover",
            "monaco.editor.action.revealDefinition",
            "monaco.editor.action.peekDefinition",
            "monaco.editor.action.goToReferences",
            "monaco.editor.action.quickOutline",
            "monaco.editor.action.gotoLine",
            "monaco.editor.action.marker.next",
            "monaco.editor.action.marker.prev",
            "monaco.editor.action.marker.nextInFiles",
            "monaco.editor.action.marker.prevInFiles",
            "monaco.editor.action.rename",
            "monaco.editor.action.quickFix",
            "monaco.editor.action.refactor",
            "monaco.editor.action.formatDocument",
            "monaco.editor.action.formatSelection",
            "monaco.editor.action.organizeImports",
            "monaco.editor.action.smartSelect.expand",
            "monaco.editor.action.smartSelect.shrink",
            "monaco.editor.action.insertCursorAbove",
            "monaco.editor.action.insertCursorBelow",
            "monaco.editor.action.insertCursorAtEndOfEachLineSelected",
            "monaco.editor.action.addSelectionToNextFindMatch",
            "monaco.editor.action.selectHighlights",
            "monaco.editor.action.duplicateSelection",
            "monaco.editor.action.copyLinesUpAction",
            "monaco.editor.action.copyLinesDownAction",
            "monaco.editor.action.moveLinesUpAction",
            "monaco.editor.action.moveLinesDownAction",
            "monaco.editor.action.deleteLines",
            "monaco.editor.action.joinLines",
            "monaco.editor.action.indentLines",
            "monaco.editor.action.outdentLines",
            "monaco.editor.action.commentLine",
            "monaco.editor.action.blockComment",
            "monaco.editor.action.transformToUppercase",
            "monaco.editor.action.transformToLowercase",
            "monaco.editor.fold",
            "monaco.editor.unfold",
            "monaco.editor.foldAll",
            "monaco.editor.unfoldAll",
            "chordPending",
            "chordNoMatch",
        ],
    ),
    (
        "preview",
        &[
            "notSupported",
            "openExternally",
            "pdf.loadFailed",
            "pdf.previousPage",
            "pdf.nextPage",
            "pdf.pageIndicator",
            "pdf.zoomIn",
            "pdf.zoomOut",
            "spreadsheet.loadFailed",
            "spreadsheet.noSheets",
            "spreadsheet.emptySheet",
            "spreadsheet.truncatedNotice",
            "hwp.loadFailed",
            "hwp.noPages",
            "hwp.previousPage",
            "hwp.nextPage",
            "hwp.pageIndicator",
            "presentation.loadFailed",
            "presentation.layoutDisclaimer",
            "presentation.slideLabel",
            "presentation.noText",
        ],
    ),
    (
        "problems",
        &[
            "title",
            "empty",
            "emptyFiltered",
            "filterAriaLabel",
            "toggleAriaLabel",
            "severity.error",
            "severity.warning",
            "severity.info",
            "severity.hint",
        ],
    ),
    ("outline", &["title", "empty", "noActiveFile"]),
    ("breadcrumbs", &["title", "noActiveFile", "dropdownAriaLabel"]),
    (
        "agent",
        &[
            "status.idle",
            "status.working",
            "status.awaitingInput",
            "status.unknown",
            "sessionTooltip",
            "hooksInstall",
            "hooksUninstall",
            "hooksInstalled",
            "hooksConsentTitle",
            "hooksConsentDescription",
            "hooksFileInvalid",
        ],
    ),
    (
        "ide",
        &[
            "title",
            "connected",
            "disconnected",
            "starting",
            "acceptChanges",
            "rejectChanges",
            "diffAccepted",
            "diffRejected",
        ],
    ),
    (
        "ai",
        &[
            "inlineEditLabel",
            "inlineEditPlaceholder",
            "inlineEditGenerating",
            "inlineEditAccept",
            "inlineEditReject",
            "inlineEditFailed",
            "inlineEditEmptyResponse",
            "inlineEditCancelled",
            "inlineEditPaletteLabel",
        ],
    ),
    (
        "remote",
        &[
            "title",
            "description",
            "enableToggle",
            "enableToggleHint",
            "statusRunning",
            "statusStopped",
            "clientCountLabel",
            "issueLink",
            "linkCopied",
            "revokeSessions",
            "sessionsRevoked",
            "startFailed",
            "securityWarning",
            "passwordLabel",
            "passwordPlaceholder",
            "passwordSet",
            "passwordClear",
            "passwordConfigured",
            "passwordNotConfigured",
            "passwordHint",
            "passwordTooShort",
            "passwordOnlyToggle",
            "passwordOnlyToggleDescription",
            "passwordSessionsRevoked",
            "passwordSaveFailed",
            "loginTitle",
            "loginPasswordLabel",
            "loginSubmit",
            "loginFailed",
            "loginLocked",
            "loginLinkExpired",
            "loginInsecureNotice",
            "allowedHostsLabel",
            "allowedHostsDescription",
            "allowedHostsPlaceholder",
            "allowedHostsAdd",
            "allowedHostsRemove",
            "allowedHostsInvalid",
            "allowedHostsSaveFailed",
        ],
    ),
    ("zen", &["hint", "hintExit"]),
    ("prompts", &["autoTabTitle", "inlineEditTitle", "commitMessageTitle", "editEntry"]),
];

pub fn required_message_keys() -> Vec<String> {
    MESSAGE_NAMESPACES
        .iter()
        .flat_map(|(namespace, keys)| keys.iter().map(move |key| format!("{namespace}.{key}")))
        .collect()
}

/// Panics when a bundled catalog fails to parse. The JSON is fixed at compile time by
/// `include_str!`, so a parse failure is a programmer error, and the `builtin_*` signatures are
/// infallible — an empty-map fallback would silently break the required-key contract enforced by
/// [`MESSAGE_NAMESPACES`]. This is a deliberate departure from both bundled-data precedents:
/// theme's `.ok()` skip (a missing optional theme degrades gracefully) and the lsp manifest's
/// empty-list fallback (a degraded feature, not a broken contract). The panic is fixed by a
/// `should_panic` test, and [`warm_builtin_catalogs`] moves its first contact to boot.
fn parse_builtin_messages(locale_id: &str, source: &str) -> BTreeMap<String, String> {
    serde_json::from_str(source).unwrap_or_else(|error| panic!("bundled locale catalog '{locale_id}' is not valid JSON: {error}"))
}

fn en_messages() -> &'static BTreeMap<String, String> {
    static MESSAGES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MESSAGES.get_or_init(|| parse_builtin_messages(BUILTIN_EN_ID, include_str!("../../../resources/locales/en.json")))
}

fn ko_messages() -> &'static BTreeMap<String, String> {
    static MESSAGES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MESSAGES.get_or_init(|| parse_builtin_messages(BUILTIN_KO_ID, include_str!("../../../resources/locales/ko.json")))
}

fn ja_messages() -> &'static BTreeMap<String, String> {
    static MESSAGES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MESSAGES.get_or_init(|| parse_builtin_messages(BUILTIN_JA_ID, include_str!("../../../resources/locales/ja.json")))
}

/// Eagerly parses all three bundled catalogs during boot, before any window is created.
/// [`parse_builtin_messages`] deliberately panics on a corrupt bundled catalog, but left lazy that
/// panic would first fire inside an async Tauri command, where it kills only the spawned task: the
/// invoke promise never settles, and the main window — revealed only after the locale query
/// settles — stays invisible forever with no diagnostic. Calling this from `setup` turns the same
/// programmer error into the loud boot-time process crash the contract intended.
pub fn warm_builtin_catalogs() {
    en_messages();
    ko_messages();
    ja_messages();
}

pub fn builtin_en() -> LocalePack {
    LocalePack {
        version: LOCALE_SCHEMA_VERSION,
        id: BUILTIN_EN_ID.to_string(),
        name: "English".to_string(),
        extends: None,
        messages: en_messages().clone(),
    }
}

pub fn builtin_ko() -> LocalePack {
    LocalePack {
        version: LOCALE_SCHEMA_VERSION,
        id: BUILTIN_KO_ID.to_string(),
        name: "한국어".to_string(),
        extends: None,
        messages: ko_messages().clone(),
    }
}

pub fn builtin_ja() -> LocalePack {
    LocalePack {
        version: LOCALE_SCHEMA_VERSION,
        id: BUILTIN_JA_ID.to_string(),
        name: "日本語".to_string(),
        extends: None,
        messages: ja_messages().clone(),
    }
}

pub fn builtin_by_id(locale_id: &str) -> Option<LocalePack> {
    match locale_id {
        BUILTIN_EN_ID => Some(builtin_en()),
        BUILTIN_KO_ID => Some(builtin_ko()),
        BUILTIN_JA_ID => Some(builtin_ja()),
        _ => None,
    }
}

fn summarize(pack: &LocalePack, builtin: bool) -> LocaleSummary {
    LocaleSummary {
        id: pack.id.clone(),
        name: pack.name.clone(),
        builtin,
    }
}

fn resolve_pack(pack: &LocalePack, base: Option<&LocalePack>, mut warnings: Vec<String>) -> ResolvedLocale {
    let mut messages = pack.messages.clone();

    if let Some(base_pack) = base {
        for (key, value) in &base_pack.messages {
            messages.entry(key.clone()).or_insert_with(|| {
                warnings.push(format!("messages.{key} filled from base locale '{}'", base_pack.id));
                value.clone()
            });
        }
    }

    ResolvedLocale {
        id: pack.id.clone(),
        name: pack.name.clone(),
        messages,
        warnings,
    }
}

pub fn list_locales(paths: &AppPaths) -> Vec<LocaleSummary> {
    let mut list = vec![
        summarize(&builtin_en(), true),
        summarize(&builtin_ko(), true),
        summarize(&builtin_ja(), true),
    ];

    let Ok(entries) = std::fs::read_dir(paths.locales_dir()) else {
        return list;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if let Ok(Some(pack)) = persist::read_json::<LocalePack>(&path) {
            match list.iter_mut().find(|summary| summary.id == pack.id) {
                Some(existing) => *existing = summarize(&pack, false),
                None => list.push(summarize(&pack, false)),
            }
        }
    }

    list
}

pub fn load_locale(paths: &AppPaths, locale_id: &str) -> AppResult<ResolvedLocale> {
    if let Some(builtin) = builtin_by_id(locale_id) {
        return Ok(resolve_pack(&builtin, None, Vec::new()));
    }

    let path = paths.locales_dir().join(format!("{locale_id}.json"));
    let pack: LocalePack = persist::read_json(&path)?.ok_or_else(|| AppError::NotFound(format!("locale not found: {locale_id}")))?;

    let mut warnings = Vec::new();
    let base = match pack.extends.as_deref() {
        Some(extends_id) => builtin_by_id(extends_id).unwrap_or_else(|| {
            warnings.push(format!(
                "unresolved locale extends '{extends_id}', falling back to '{BUILTIN_EN_ID}'"
            ));
            builtin_en()
        }),
        None => builtin_en(),
    };

    Ok(resolve_pack(&pack, Some(&base), warnings))
}

pub fn locale_exists(paths: &AppPaths, locale_id: &str) -> bool {
    builtin_by_id(locale_id).is_some() || paths.locales_dir().join(format!("{locale_id}.json")).exists()
}

pub fn save_locale(paths: &AppPaths, pack: &LocalePack) -> AppResult<LocaleSummary> {
    if pack.id.trim().is_empty() {
        return Err(AppError::InvalidArgument("locale id must not be empty".to_string()));
    }
    if builtin_by_id(&pack.id).is_some() {
        return Err(AppError::InvalidArgument(format!("cannot overwrite builtin locale: {}", pack.id)));
    }
    if pack.id.contains(['/', '\\', '.']) {
        return Err(AppError::InvalidArgument(format!("invalid locale id: {}", pack.id)));
    }

    std::fs::create_dir_all(paths.locales_dir())?;
    persist::write_json(&paths.locales_dir().join(format!("{}.json", pack.id)), pack)?;
    Ok(summarize(pack, false))
}

pub fn resolve_language(paths: &AppPaths, language: &str, system_language: &str) -> String {
    if language != "system" {
        if locale_exists(paths, language) {
            return language.to_string();
        }
        return BUILTIN_EN_ID.to_string();
    }

    let lower = system_language.to_lowercase();
    let prefix = lower.get(0..2).unwrap_or("");

    match prefix {
        "ko" => BUILTIN_KO_ID.to_string(),
        "ja" => BUILTIN_JA_ID.to_string(),
        _ => BUILTIN_EN_ID.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-locale-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn 내장_3종_로케일은_같은_키_집합을_가진다() {
        let en_keys: std::collections::BTreeSet<_> = builtin_en().messages.keys().cloned().collect();
        let ko_keys: std::collections::BTreeSet<_> = builtin_ko().messages.keys().cloned().collect();
        let ja_keys: std::collections::BTreeSet<_> = builtin_ja().messages.keys().cloned().collect();

        assert_eq!(en_keys, ko_keys);
        assert_eq!(en_keys, ja_keys);

        for key in required_message_keys() {
            assert!(en_keys.contains(&key), "missing message key: {key}");
        }
    }

    #[test]
    fn en_메시지의_모든_키는_required_message_keys에_포함된다() {
        let en_keys: std::collections::BTreeSet<_> = builtin_en().messages.keys().cloned().collect();
        let required_keys: std::collections::BTreeSet<_> = required_message_keys().into_iter().collect();

        for key in &en_keys {
            assert!(required_keys.contains(key), "MESSAGE_NAMESPACES에 등록되지 않은 키: {key}");
        }
    }

    #[test]
    fn 내장_카탈로그는_한_번만_파싱되어_캐시된다() {
        assert!(std::ptr::eq(en_messages(), en_messages()));
        assert!(std::ptr::eq(ko_messages(), ko_messages()));
        assert!(std::ptr::eq(ja_messages(), ja_messages()));
    }

    #[test]
    fn 내장_카탈로그_대표_값이_언어별로_정확하다() {
        assert_eq!(builtin_en().messages.get("common.cancel").map(String::as_str), Some("Cancel"));
        assert_eq!(builtin_ko().messages.get("common.cancel").map(String::as_str), Some("취소"));
        assert_eq!(builtin_ja().messages.get("common.cancel").map(String::as_str), Some("キャンセル"));
        assert_eq!(
            builtin_en().messages.get("explorer.deleteConfirmTitle").map(String::as_str),
            Some("Delete {{name}}?")
        );
        assert_eq!(
            builtin_en().messages.get("snippetEditor.invalidFileName").map(String::as_str),
            Some("This file name isn't valid — avoid /, \\, .., and :")
        );
        assert_eq!(
            builtin_ko().messages.get("snippetEditor.invalidFileName").map(String::as_str),
            Some("올바른 파일 이름이 아닙니다 — /, \\, .., : 는 사용할 수 없습니다")
        );
        assert_eq!(
            builtin_ja()
                .messages
                .get("settings.pluginUninstallConfirmTitle")
                .map(String::as_str),
            Some("{{name}} をアンインストールしますか？")
        );
    }

    #[test]
    #[should_panic(expected = "is not valid JSON")]
    fn 손상된_내장_카탈로그는_패닉한다() {
        parse_builtin_messages(BUILTIN_EN_ID, "{");
    }

    #[test]
    fn 내장_카탈로그_원문에_중복_키가_없다() {
        const CATALOG_ENTRY_LINE_PREFIX: &str = "    \"";
        for (locale_id, source, parsed_len) in [
            (
                BUILTIN_EN_ID,
                include_str!("../../../resources/locales/en.json"),
                en_messages().len(),
            ),
            (
                BUILTIN_KO_ID,
                include_str!("../../../resources/locales/ko.json"),
                ko_messages().len(),
            ),
            (
                BUILTIN_JA_ID,
                include_str!("../../../resources/locales/ja.json"),
                ja_messages().len(),
            ),
        ] {
            let raw_entry_count = source.lines().filter(|line| line.starts_with(CATALOG_ENTRY_LINE_PREFIX)).count();
            assert_eq!(
                raw_entry_count, parsed_len,
                "bundled catalog '{locale_id}' raw entry count differs from parsed map size — a duplicate key was silently collapsed by serde (or the prettier one-entry-per-line format changed)"
            );
        }
    }

    #[test]
    fn extends_병합이_누락_키를_base로_채운다() {
        let data_dir = temp_data_dir("extends-merge");
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(paths.locales_dir()).expect("create locales dir");

        let mut messages = BTreeMap::new();
        messages.insert("common.cancel".to_string(), "그만".to_string());
        let custom = LocalePack {
            version: LOCALE_SCHEMA_VERSION,
            id: "custom-ko".to_string(),
            name: "Custom Korean".to_string(),
            extends: Some(BUILTIN_KO_ID.to_string()),
            messages,
        };
        persist::write_json(&paths.locales_dir().join("custom-ko.json"), &custom).expect("write custom locale");

        let resolved = load_locale(&paths, "custom-ko").expect("load custom locale");

        assert_eq!(resolved.messages.get("common.cancel"), Some(&"그만".to_string()));
        assert_eq!(resolved.messages.get("common.save"), builtin_ko().messages.get("common.save"));
        assert!(!resolved.warnings.is_empty());
        for key in required_message_keys() {
            assert!(resolved.messages.contains_key(&key));
        }

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }

    #[test]
    fn 해석_불가_extends는_경고를_남기고_en으로_폴백한다() {
        let data_dir = temp_data_dir("extends-unresolved");
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(paths.locales_dir()).expect("create locales dir");

        let custom = LocalePack {
            version: LOCALE_SCHEMA_VERSION,
            id: "custom-broken".to_string(),
            name: "Broken".to_string(),
            extends: Some("does-not-exist".to_string()),
            messages: BTreeMap::new(),
        };
        persist::write_json(&paths.locales_dir().join("custom-broken.json"), &custom).expect("write broken locale");

        let resolved = load_locale(&paths, "custom-broken").expect("load broken locale");

        assert_eq!(resolved.messages.get("common.cancel"), builtin_en().messages.get("common.cancel"));
        assert!(resolved
            .warnings
            .iter()
            .any(|warning| warning.contains("unresolved locale extends")));

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }

    #[test]
    fn 없는_로케일_아이디는_notfound를_반환한다() {
        let paths = AppPaths::new(temp_data_dir("missing"));
        let result = load_locale(&paths, "does-not-exist");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[test]
    fn list_locales는_내장_3종과_사용자_팩을_반환하고_겹치면_사용자_팩이_이긴다() {
        let data_dir = temp_data_dir("list");
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(paths.locales_dir()).expect("create locales dir");

        let mut overriding_en = builtin_en();
        overriding_en.name = "English (custom)".to_string();
        persist::write_json(&paths.locales_dir().join("en.json"), &overriding_en).expect("write overriding pack");

        let list = list_locales(&paths);

        assert_eq!(list.len(), 3);
        let en_summary = list.iter().find(|summary| summary.id == BUILTIN_EN_ID).expect("en summary exists");
        assert!(!en_summary.builtin);
        assert_eq!(en_summary.name, "English (custom)");

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }

    #[test]
    fn resolve_language는_system을_시스템_언어로_매핑한다() {
        let paths = AppPaths::new(temp_data_dir("resolve-system"));
        assert_eq!(resolve_language(&paths, "system", "ko-KR"), BUILTIN_KO_ID);
        assert_eq!(resolve_language(&paths, "system", "ja-JP"), BUILTIN_JA_ID);
        assert_eq!(resolve_language(&paths, "system", "fr-FR"), BUILTIN_EN_ID);
    }

    #[test]
    fn resolve_language는_명시된_언어를_그대로_사용한다() {
        let paths = AppPaths::new(temp_data_dir("resolve-explicit"));
        assert_eq!(resolve_language(&paths, BUILTIN_JA_ID, "en-US"), BUILTIN_JA_ID);
    }

    #[test]
    fn 내장_로케일_아이디로는_저장할_수_없다() {
        let paths = AppPaths::new(temp_data_dir("save-builtin"));
        let mut pack = builtin_en();
        pack.id = BUILTIN_EN_ID.to_string();
        assert!(save_locale(&paths, &pack).is_err());
    }

    #[test]
    fn 경로_구분자가_섞인_아이디는_저장을_거부한다() {
        let paths = AppPaths::new(temp_data_dir("save-path"));
        let mut pack = builtin_en();
        pack.id = "../evil".to_string();
        assert!(save_locale(&paths, &pack).is_err());
    }

    #[test]
    fn 사용자_로케일_팩은_저장하고_목록에_나타난다() {
        let data_dir = temp_data_dir("save-ok");
        let paths = AppPaths::new(data_dir);
        let mut pack = builtin_en();
        pack.id = "my-locale".to_string();
        pack.name = "My Locale".to_string();
        pack.extends = Some(BUILTIN_EN_ID.to_string());

        let summary = save_locale(&paths, &pack).expect("save");

        assert!(!summary.builtin);
        assert!(list_locales(&paths).iter().any(|item| item.id == "my-locale"));

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }

    #[test]
    fn resolve_language는_존재하지_않는_언어를_en으로_폴백한다() {
        let paths = AppPaths::new(temp_data_dir("resolve-missing"));
        assert_eq!(resolve_language(&paths, "xx", "en-US"), BUILTIN_EN_ID);
    }
}
