import { invokeIpc } from '../lib/ipc'
import { openFileViaQuickOpen } from '../lib/palette'
import { STORAGE_STATE_PATH } from '../lib/paths'
import { expect, test } from '../lib/taide-fixture'

test('한 원격 세션에서 연 탭이 다른 원격 세션의 탭바·layout_get 에도 즉시 반영된다', async ({ page, browser, taideBaseUrl, fixtureProject }) => {
    const secondContext = await browser.newContext({ storageState: STORAGE_STATE_PATH })
    const secondPage = await secondContext.newPage()
    await secondPage.goto(taideBaseUrl)

    try {
        await openFileViaQuickOpen(page, 'index.ts')

        await expect(secondPage.getByRole('tab', { name: /index\.ts/ })).toBeVisible()

        await expect(async () => {
            const layoutOnSecond = await invokeIpc(secondPage, 'layout_get', { projectId: fixtureProject.projectId })
            expect(JSON.stringify(layoutOnSecond)).toContain(fixtureProject.rootDir)
        }).toPass()
    } finally {
        await secondContext.close()
    }
})
