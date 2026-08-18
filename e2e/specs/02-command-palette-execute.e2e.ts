import { KEY_CHORD } from '../lib/constants'
import { expect, test } from '../lib/taide-fixture'

test('팔레트에서 Toggle Sidebar 커맨드를 실행하면 옵션 목록이 뜨고 사이드바가 토글된다', async ({ page, fixtureProject }) => {
    void fixtureProject

    const explorerTree = page.getByRole('tree')
    await expect(explorerTree).toBeVisible()

    await page.keyboard.press(KEY_CHORD.COMMAND_PALETTE)
    const dialog = page.getByRole('dialog')
    await dialog.waitFor({ state: 'visible' })
    await dialog.locator('input').first().fill('>toggle sidebar')

    const options = page.getByRole('option')
    await expect(options.first()).toBeVisible()
    await expect(options.first()).toContainText('Toggle Sidebar')
    expect(await options.count()).toBeGreaterThanOrEqual(1)

    await options.first().click()
    await expect(explorerTree).toBeHidden()

    await page.keyboard.press(KEY_CHORD.COMMAND_PALETTE)
    await dialog.locator('input').first().fill('>toggle sidebar')
    await options.first().click()
    await expect(explorerTree).toBeVisible()
})
