/**
 * Pull-based context sources for keymap `when` evaluation (`keymap-when.ts`). Each getter is
 * read synchronously at keydown time rather than pushed/registered ahead of time — there is no
 * monaco `IContextKeyService` instance backing this (see the Wave H contract §3.2, "평가기만
 * 재사용, 소스는 pull 게터"), so context values are derived fresh from the live DOM on every check.
 */
export type KeymapContextGetters = Record<string, () => boolean>

const isActiveElementWithin = (selector: string) => Boolean(document.activeElement?.closest(selector))

/**
 * First-pass context whitelist (Wave H §3.2): whether the focused element sits inside a monaco
 * editor instance ('.monaco-editor' — monaco's own root widget class) or an xterm.js terminal
 * ('.xterm' — xterm's own root class). Both are real, stable classes monaco/xterm apply to their
 * own container regardless of which specific editor/terminal instance is focused, which is exactly
 * what a *global* when-context needs (there is no per-instance scoping here — see the
 * multi-terminal-pane caveat noted for `terminalFocus` consumers in the Phase B handoff).
 *
 * `editorTextFocus` here is deliberately broader than monaco's own same-named context key
 * (`EditorContextKeys.textInputFocus`, which is scoped to the `textarea.inputarea` element
 * specifically and reads `false` while the Find widget, Rename input, or any other in-editor overlay
 * has focus). This module's definition — "inside the editor's container at all" — is the one Wave H
 * contract §3.2 specifies for chord-deferral/dispatch gating; don't narrow it to match monaco's
 * semantics without re-confirming that contract, since several call sites (`keymap-dispatch.ts`'s
 * chord-prefix gate) rely on the wider container-level reading.
 */
export const DEFAULT_KEYMAP_CONTEXT_GETTERS: KeymapContextGetters = {
    editorTextFocus: () => isActiveElementWithin('.monaco-editor'),
    terminalFocus: () => isActiveElementWithin('.xterm'),
}

export const getKeymapContextValue = (key: string, getters: KeymapContextGetters = DEFAULT_KEYMAP_CONTEXT_GETTERS) => getters[key]?.() ?? false
