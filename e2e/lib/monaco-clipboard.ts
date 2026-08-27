import type { Locator, Page } from '@playwright/test'
import { KEY_CHORD } from './constants'

/**
 * Replaces the full content of a focused Monaco editor by writing `text` to the system clipboard
 * and pasting it, instead of `page.keyboard.type()`/`insertText()`. Both of those corrupt
 * multi-line, bracket-heavy content (e.g. a `JSON.stringify(..., null, 4)` settings body) on this
 * codebase's WebKit target:
 * - `type()` dispatches real per-key events, so Monaco's auto-indent-on-Enter re-indents every
 *   typed newline *relative to the already-indented text this harness is typing* — reproduced:
 *   a 4-space-indented JSON literal grows another ~4 spaces of indentation per line — and its
 *   auto-closing-bracket "type over" tracking loses track of which trailing `}`/`"` were
 *   auto-inserted across that much intervening input, leaving a duplicate trailing `}` behind.
 * - `insertText()` sidesteps the per-key auto-indent path but reproduces the already-documented
 *   Monaco/WKWebView composition-event bug instead (`docs/bug/2026-08-06-wkwebview-ime-composition.md`,
 *   `docs/bug/2026-08-12-editor-korean-ime.md`) — verified: stray spaces appear around ordinary
 *   punctuation even in single-line ASCII text.
 *
 * A real paste inserts `text` byte-for-byte in one model edit, avoiding both — every editor this
 * harness drives has `formatOnPaste` off, so nothing reformats it afterward either (verified: a
 * paste-then-copy round trip through the system clipboard reproduces `text` exactly).
 */
export const replaceEditorContentViaPaste = async (page: Page, editor: Locator, text: string) => {
    await editor.click()
    await page.keyboard.press(KEY_CHORD.SELECT_ALL)
    await page.evaluate((value) => navigator.clipboard.writeText(value), text)
    await page.keyboard.press(KEY_CHORD.PASTE)
}
