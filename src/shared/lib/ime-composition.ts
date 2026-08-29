/**
 * Legacy `KeyboardEvent.keyCode` (`VK_PROCESSKEY`) every engine reports for a keydown an IME is
 * consuming — the composition-in-progress signal that still works in this app's runtime.
 *
 * `KeyboardEvent.isComposing` alone cannot be trusted here: Tauri 2's macOS WKWebView never fires
 * `compositionstart`/`update`/`end` at all, so `isComposing` is permanently `false` for Korean/
 * Japanese/Chinese input while `keyCode` still reports 229 exactly as Safari does
 * (`docs/bug/2026-08-06-wkwebview-ime-composition.md`, measured side by side). Every keydown guard
 * written against `isComposing` alone is therefore dead code in the shipped app, which is why
 * `isImeCompositionKeydown` — not a bare `event.isComposing` — is the app's single guard.
 */
export const IME_COMPOSITION_KEY_CODE = 229

/**
 * Structural shape covering both event flavors a keydown handler can receive, so one guard serves
 * `window`/element native listeners and React's synthetic `onKeyDown` alike:
 *
 * - native `KeyboardEvent` carries `isComposing` and `keyCode` directly;
 * - React's synthetic keyboard event carries `keyCode` but exposes `isComposing` only through
 *   `nativeEvent` (see `@types/react`'s `KeyboardEvent`, which omits it).
 */
type ImeCompositionKeydown = {
    isComposing?: boolean
    keyCode?: number
    nativeEvent?: { isComposing?: boolean }
}

/**
 * `true` when this keydown belongs to an in-flight IME composition and must not be treated as the
 * user pressing that key — the same test cmdk applies before acting on any key
 * (`nativeEvent.isComposing || keyCode === 229`). Guarding on it keeps composing text from
 * triggering Enter-submit handlers, Escape-dismiss handlers, or keymap dispatch/chord resolution
 * with the half-formed syllable the user is still typing.
 *
 * The `isComposing` legs stay ahead of the `keyCode` one so the guard keeps holding on any runtime
 * that does fire composition events (a fixed WKWebView, a browser-based dev/e2e run) even if that
 * runtime ever stops reporting the deprecated `keyCode`.
 */
export const isImeCompositionKeydown = (event: ImeCompositionKeydown) =>
    event.isComposing === true || event.nativeEvent?.isComposing === true || event.keyCode === IME_COMPOSITION_KEY_CODE
