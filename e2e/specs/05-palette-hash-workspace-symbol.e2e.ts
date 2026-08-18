import path from 'node:path'
import { KEY_CHORD, LSP_SYMBOL_TIMEOUT_MS } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { openFileViaQuickOpen } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

test('워크스페이스 심볼(#)로 다른 파일의 심볼을 선택하면 그 파일 탭이 열린다', async ({ page, fixtureProject }) => {
    await openFileViaQuickOpen(page, 'index.ts')

    await page.keyboard.press(KEY_CHORD.WORKSPACE_SYMBOL)
    const dialog = page.getByRole('dialog')
    await dialog.waitFor({ state: 'visible' })
    await dialog.locator('input').first().fill('#shout')

    const symbolOption = page.getByRole('option').filter({ hasText: 'shout' }).first()
    await expect(symbolOption).toBeVisible({ timeout: LSP_SYMBOL_TIMEOUT_MS })
    await symbolOption.click()

    await expect(page.getByRole('tab', { name: /other\.ts/ })).toBeVisible()

    const otherFilePath = path.join(fixtureProject.rootDir, 'src/other.ts')
    await expect(async () => {
        const layout = await invokeIpc(page, 'layout_get', { projectId: fixtureProject.projectId })
        expect(JSON.stringify(layout)).toContain(otherFilePath)
    }).toPass()
})
