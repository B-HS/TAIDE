import type { Page } from '@playwright/test'
import { invokeIpc } from '../lib/ipc'
import { runPaletteCommand } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

const EDITOR_BACKGROUND_VAR = '--taide-editor-background'

const readEditorBackgroundVar = (page: Page) =>
    page.evaluate((varName) => getComputedStyle(document.documentElement).getPropertyValue(varName), EDITOR_BACKGROUND_VAR)

test('테마를 전환하면 CSS 변수와 settings.themeId 가 반영되고, 되돌리면 원복된다', async ({ page, fixtureProject }) => {
    void fixtureProject
    await runPaletteCommand(page, 'App: Settings', { exact: true })

    const activeButton = page.locator('button[aria-pressed="true"]').first()
    await expect(activeButton).toBeVisible()
    const originalThemeName = await activeButton.innerText()
    const backgroundBefore = await readEditorBackgroundVar(page)

    const themeIdBefore = (await invokeIpc<{ themeId?: string }>(page, 'settings_get')).themeId

    const otherButton = page.locator('button[aria-pressed="false"]').first()
    await otherButton.click()

    await expect(async () => {
        const settings = await invokeIpc<{ themeId?: string }>(page, 'settings_get')
        expect(settings.themeId).toBeTruthy()
        expect(settings.themeId).not.toBe(themeIdBefore)
    }).toPass()

    const backgroundAfter = await readEditorBackgroundVar(page)
    expect(backgroundAfter).not.toBe(backgroundBefore)

    const revertButton = page.locator('button[aria-pressed="false"]').filter({ hasText: originalThemeName }).first()
    await revertButton.click()

    await expect(async () => {
        const backgroundRestored = await readEditorBackgroundVar(page)
        expect(backgroundRestored).toBe(backgroundBefore)
        const settings = await invokeIpc<{ themeId?: string }>(page, 'settings_get')
        expect(settings.themeId).toBe(themeIdBefore)
    }).toPass()
})
