import { KEY_CHORD, LSP_SYMBOL_TIMEOUT_MS, PALETTE_MODE_PREFIX } from '../lib/constants'
import { openFileViaQuickOpen } from '../lib/palette'
import { cursorPositionLabel } from '../lib/status-bar'
import { expect, test } from '../lib/taide-fixture'

/**
 * Pilot risk (design doc § 리스크 5): document-symbol results depend on an LSP session attaching
 * to the `.ts` model. If no TypeScript language server is configured for a bare fixture folder
 * (no `tsconfig.json`/`package.json`), this option list may stay empty — that is the exact
 * condition this spec's generous `LSP_SYMBOL_TIMEOUT_MS` wait is meant to surface, not paper over.
 */
test('팔레트 @ 모드로 문서 심볼을 검색하면 옵션 목록이 뜨고 커서가 심볼 줄로 이동한다', async ({ page, fixtureProject }) => {
    void fixtureProject
    await openFileViaQuickOpen(page, 'index.ts')

    await page.keyboard.press(KEY_CHORD.QUICK_OPEN)
    const dialog = page.getByRole('dialog')
    await dialog.waitFor({ state: 'visible' })
    await dialog.locator('input').first().fill(`${PALETTE_MODE_PREFIX.SYMBOL}greet`)

    const symbolOption = page.getByRole('option').filter({ hasText: 'greet' }).first()
    await expect(symbolOption).toBeVisible({ timeout: LSP_SYMBOL_TIMEOUT_MS })

    await symbolOption.click()
    await expect(cursorPositionLabel(page)).toContainText(/Ln 1,/)
})
