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
    await page.keyboard.press('Enter')
    /**
     * `page.keyboard.type()`, not `insertText()` — under this codebase's WebKit target,
     * `insertText()` reproduces the already-documented Monaco/WKWebView composition-event bug
     * (`docs/bug/2026-08-06-wkwebview-ime-composition.md`), corrupting plain ASCII text (verified:
     * stray spaces appear around ordinary punctuation like `=`/`-`). `type()` dispatches real
     * per-key events instead, which this single-line insertion (no newlines, so no auto-indent
     * compounding) types correctly.
     */
    await page.keyboard.type(SAVE_MARKER_LINE)

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
