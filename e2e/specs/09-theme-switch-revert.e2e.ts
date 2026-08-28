import type { Page } from '@playwright/test'
import { invokeIpc } from '../lib/ipc'
import { runPaletteCommand } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

const EDITOR_BACKGROUND_VAR = '--taide-editor-background'
const BUILTIN_DARK_THEME_NAME = 'TAIDE Dark'
const BUILTIN_LIGHT_THEME_NAME = 'TAIDE Light'

const readEditorBackgroundVar = (page: Page) =>
    page.evaluate((varName) => getComputedStyle(document.documentElement).getPropertyValue(varName), EDITOR_BACKGROUND_VAR)

/**
 * This harness never resets the developer's real theme choice before a run (only `globalTeardown`
 * restores it, after the whole suite finishes — `docs/quality-assurance/2026-08-18-e2e-harness.md`
 * §3), so the theme active when this spec starts is whatever the developer happened to be using —
 * it must not be assumed to be a specific one, or even the first `aria-pressed="false"` button in
 * DOM order (that assumption broke when the active theme was `Darcula`, far down the "Bundled
 * Themes" grid: switching *to* the first inactive button worked, but the later revert click had to
 * scroll a long single-scroll Settings page back down to that same button). The two `builtin` TAIDE
 * themes are always rendered first, so picking whichever of *those* two isn't currently active
 * gives a switch target that never needs scrolling, regardless of what the active theme actually is.
 */
test('테마를 전환하면 CSS 변수와 settings.themeId 가 반영되고, 되돌리면 원복된다', async ({ page, fixtureProject }) => {
    void fixtureProject
    await runPaletteCommand(page, 'App: Settings', { exact: true })

    const builtinDarkButton = page.getByRole('button', { name: BUILTIN_DARK_THEME_NAME })
    const builtinLightButton = page.getByRole('button', { name: BUILTIN_LIGHT_THEME_NAME })
    await expect(builtinDarkButton).toBeVisible()
    await expect(builtinLightButton).toBeVisible()

    const activeButton = page.locator('button[aria-pressed="true"]').first()
    await expect(activeButton).toBeVisible()
    /**
     * First line only: a theme button's `innerText` is multiline ("<name>\n<Dark|Light badge>"),
     * but Playwright's `filter({ hasText })` matches against `textContent`, where those two spans
     * concatenate with no separator ("Monokai DimmedDark") — a multiline needle can never match
     * it, so the revert locator below would resolve to zero elements forever (reproduced on the
     * first end-to-end clean run of this spec, 2026-08-27). The name line alone is unique enough:
     * even when the original theme is one of the two builtins, the aria-pressed filter picks the
     * right button.
     */
    const originalThemeName = (await activeButton.innerText()).split('\n')[0]
    const backgroundBefore = await readEditorBackgroundVar(page)

    const themeIdBefore = (await invokeIpc<{ themeId?: string }>(page, 'settings_get')).themeId

    const isDarkActive = (await builtinDarkButton.getAttribute('aria-pressed')) === 'true'
    const switchTarget = isDarkActive ? builtinLightButton : builtinDarkButton
    await switchTarget.click()

    await expect(async () => {
        const settings = await invokeIpc<{ themeId?: string }>(page, 'settings_get')
        expect(settings.themeId).toBeTruthy()
        expect(settings.themeId).not.toBe(themeIdBefore)
    }).toPass()

    const backgroundAfter = await readEditorBackgroundVar(page)
    expect(backgroundAfter).not.toBe(backgroundBefore)

    const revertButton = page.locator('button[aria-pressed="false"]').filter({ hasText: originalThemeName }).first()
    await revertButton.scrollIntoViewIfNeeded()
    await revertButton.click()

    await expect(async () => {
        const backgroundRestored = await readEditorBackgroundVar(page)
        expect(backgroundRestored).toBe(backgroundBefore)
        const settings = await invokeIpc<{ themeId?: string }>(page, 'settings_get')
        expect(settings.themeId).toBe(themeIdBefore)
    }).toPass()
})
