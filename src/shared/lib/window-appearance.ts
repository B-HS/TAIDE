import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ThemeType } from '@shared/api/bindings'

/**
 * Forwards `type` straight to tao's `Window::set_theme` (`getCurrentWindow().setTheme`) — a native
 * IPC round-trip. Callers must not invoke this on every value change without first checking whether
 * `type` actually differs from the last one applied: tao 0.35.3's `set_ns_theme` unconditionally
 * calls `NSApplication.setAppearance:` (a whole-app appearance repass) with no same-value
 * short-circuit (contract d-45 §0, `platform_impl/macos/window.rs:384`), so a caller that
 * re-invokes this per pointermove during a color-picker drag floods the main thread with redundant
 * native calls even though `type` never actually changes mid-drag. `ThemeProvider` — currently the
 * sole consumer — guards this behind a last-applied-type ref for exactly that reason (contract d-45
 * §1#1); a second consumer should gate here instead, at the module level.
 *
 * Returns the underlying `setTheme` promise (contract d-45 F-02) so a caller that optimistically
 * records `type` as applied before invoking this can roll that record back on rejection — otherwise
 * a transient failure (window not yet ready, a plugin/ACL error, a remote page where this IPC axis
 * has no effect) would leave the guard believing `type` is already applied when the native call
 * never actually landed, permanently blocking every later retry of the same `type`.
 */
export const applyWindowAppearance = (type: ThemeType) => getCurrentWindow().setTheme(type)
