import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { KEY_CHORD } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { openFileViaQuickOpen } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

const SAVE_MARKER_LINE = "export const e2eSaveMarker = 'taide-e2e-save-round-trip'"

type OpenedFile = { path: string; content: string }

test('저장한 내용이 디스크와 file_open 재조회 모두에 반영된다', async ({ page, fixtureProject }) => {
    await openFileViaQuickOpen(page, 'index.ts')

    const editor = page.locator('.monaco-editor').first()
    await editor.click()
    await page.keyboard.press('Meta+End')
    /**
     * Clipboard paste, not `type()` or `insertText()` — under this codebase's WebKit target both
     * per-key paths reproduce the documented Monaco/WKWebView composition-event bug
     * (`docs/bug/2026-08-06-wkwebview-ime-composition.md`): `insertText()` corrupts plain ASCII
     * deterministically, and `type()` — this spec's previous choice — was observed corrupting it
     * intermittently too (reproduced 2026-08-27: the typed marker landed on disk as
     * `'save - round - trip'`, stray spaces around every `-`). A paste inserts the text in one
     * model edit, byte-for-byte (same rationale as `lib/monaco-clipboard.ts`, which specs 10·11
     * already rely on); the leading `\n` rides inside the pasted payload so no `Enter` keypress —
     * and no auto-indent — is involved either.
     */
    await page.evaluate((line) => navigator.clipboard.writeText(`\n${line}`), SAVE_MARKER_LINE)
    await page.keyboard.press(KEY_CHORD.PASTE)

    const filePath = path.join(fixtureProject.rootDir, 'index.ts')
    const tab = page.getByRole('tab', { name: /index\.ts/ })
    await expect(tab.locator('.bg-tab-bar-dirty-dot')).toBeVisible()

    await page.keyboard.press(KEY_CHORD.SAVE)
    await expect(tab.locator('.bg-tab-bar-dirty-dot')).toBeHidden()

    await expect(async () => {
        const diskContent = await readFile(filePath, 'utf8')
        expect(diskContent).toContain(SAVE_MARKER_LINE)
    }).toPass()

    const reopened = await invokeIpc<OpenedFile>(page, 'file_open', { path: filePath })
    expect(reopened.content).toContain(SAVE_MARKER_LINE)
})
