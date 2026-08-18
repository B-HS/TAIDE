import { KEY_CHORD, PALETTE_MODE_PREFIX } from '../lib/constants'
import { openFileViaQuickOpen } from '../lib/palette'
import { cursorPositionLabel } from '../lib/status-bar'
import { expect, test } from '../lib/taide-fixture'

test('활성 파일이 없으면 : 모드는 옵션 없이 무해하게 종료된다', async ({ page, fixtureProject }) => {
    void fixtureProject
    await page.keyboard.press(KEY_CHORD.QUICK_OPEN)
    const dialog = page.getByRole('dialog')
    await dialog.waitFor({ state: 'visible' })
    await dialog.locator('input').first().fill(`${PALETTE_MODE_PREFIX.LINE}2`)

    await expect(page.getByRole('option')).toHaveCount(0)
    await page.keyboard.press('Escape')
})

test('팔레트 : 모드로 줄 번호를 입력하면 해당 줄로 이동한다', async ({ page, fixtureProject }) => {
    void fixtureProject
    await openFileViaQuickOpen(page, 'index.ts')

    await page.keyboard.press(KEY_CHORD.QUICK_OPEN)
    const dialog = page.getByRole('dialog')
    await dialog.waitFor({ state: 'visible' })
    await dialog.locator('input').first().fill(`${PALETTE_MODE_PREFIX.LINE}2`)

    const lineOption = page.getByRole('option').first()
    await expect(lineOption).toBeVisible()
    await expect(lineOption).toContainText('2')

    await lineOption.click()
    await expect(cursorPositionLabel(page)).toContainText('Ln 2')
})
