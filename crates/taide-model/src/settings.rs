use serde::{Deserialize, Serialize};
use specta::Type;

/// `Settings.editorRenderWhitespace` / `SettingsPatch.editorRenderWhitespace`'s value set — mirrors
/// `EditorRenderWhitespace` (`src/features/editor/code-editor.tsx`), the Monaco `renderWhitespace`
/// option's own literal union. Was a bare `Option<String>` (audit
/// `docs/quality-assurance/2026-08-18-architecture-audit.md` C6/T1-B), which forced 9 frontend `as`
/// casts and let a hand-edited `settings.json`/synced gist carry an arbitrary string. Narrowing to a
/// real enum makes specta emit the same `"none" | "boundary" | "selection" | "all"` union the
/// frontend already hand-declared, so those casts collapse to a plain type. A legacy/out-of-range
/// value is normalized back to the default *before* typed parse — see
/// `domain::settings::service::sanitize_legacy_settings_values`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum EditorRenderWhitespace {
    None,
    Boundary,
    #[default]
    Selection,
    All,
}

/// `Settings.editorCursorStyle` / `SettingsPatch.editorCursorStyle`'s value set — mirrors
/// `EditorCursorStyle` (`src/features/editor/code-editor.tsx`), Monaco's `cursorStyle` option. See
/// [`EditorRenderWhitespace`]'s doc comment for why this is a narrowed enum rather than `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum EditorCursorStyle {
    #[default]
    Line,
    Block,
    Underline,
}

/// `Settings.editorCursorBlinking` / `SettingsPatch.editorCursorBlinking`'s value set — mirrors
/// `EditorCursorBlinkingStyle` (`src/features/editor/code-editor.tsx`), Monaco's `cursorBlinking`
/// option. See [`EditorRenderWhitespace`]'s doc comment for why this is a narrowed enum rather than
/// `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum EditorCursorBlinking {
    #[default]
    Blink,
    Smooth,
    Phase,
    Expand,
    Solid,
}

/// `Settings.terminalCursorStyle` / `SettingsPatch.terminalCursorStyle`'s value set — mirrors
/// `TerminalCursorStyle` (`src/features/terminal/terminal-view.tsx`), xterm.js's `cursorStyle`
/// option. See [`EditorRenderWhitespace`]'s doc comment for why this is a narrowed enum rather than
/// `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum TerminalCursorStyle {
    #[default]
    Bar,
    Block,
    Underline,
}
