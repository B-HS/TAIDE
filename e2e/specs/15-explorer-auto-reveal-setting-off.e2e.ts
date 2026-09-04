import type { Page } from '@playwright/test'
import { explorerTreeRow } from '../lib/explorer'
import { invokeIpc } from '../lib/ipc'
import { openFileViaQuickOpen, runPaletteCommand } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

const AUTO_REVEAL_SWITCH_LABEL = 'Auto reveal active file in Explorer'
const SETTINGS_COMMAND_LABEL = 'App: Settings'

/**
 * The switch lives in the Settings tab's Interface section (`settings-interface-section.tsx`,
 * `SwitchField` → Radix `Switch`, `role='switch'`). `SwitchField` wraps the control in a `<label>`
 * whose text is the label plus its description, so the accessible name is matched as a substring.
 * Settings is one long scrolling page (spec 09), hence the explicit scroll before clicking.
 */
const setAutoRevealSwitch = async (page: Page, checked: boolean) => {
    await runPaletteCommand(page, SETTINGS_COMMAND_LABEL, { exact: true })
    const autoRevealSwitch = page.getByRole('switch', { name: AUTO_REVEAL_SWITCH_LABEL })
    await autoRevealSwitch.scrollIntoViewIfNeeded()
    await expect(autoRevealSwitch).toBeVisible()
    if ((await autoRevealSwitch.getAttribute('aria-checked')) !== String(checked)) await autoRevealSwitch.click()
    await expect(autoRevealSwitch).toHaveAttribute('aria-checked', String(checked))
    await expect(async () => {
        const settings = await invokeIpc<{ explorerAutoReveal?: boolean }>(page, 'settings_get')
        expect(settings.explorerAutoReveal ?? true).toBe(checked)
    }).toPass()
}

/**
 * Counterpart of spec 14: with `explorerAutoReveal` switched off through the settings UI, opening
 * `src/nested-only.ts` via quick-open must leave the never-expanded `src/` directory alone, so its
 * row never appears in the tree (`decideAutoReveal` → `skip`). `globalTeardown` restores the whole
 * settings snapshot anyway, but the switch is turned back on at the end of the spec too so a later
 * spec in the same run (or spec 14 on a re-run) does not inherit the disabled state.
 */
test('설정에서 자동 reveal 을 끄면 퀵오픈으로 파일을 열어도 파일 트리가 펼쳐지지 않는다', async ({ page, fixtureProject }) => {
    void fixtureProject

    await expect(page.getByRole('tree')).toBeVisible()
    await setAutoRevealSwitch(page, false)

    try {
        await openFileViaQuickOpen(page, 'nested-only')
        await expect(page.getByRole('tab', { name: /nested-only\.ts/ })).toBeVisible()
        await expect(page.locator('.monaco-editor').first()).toContainText('nestedOnly')

        await expect(explorerTreeRow(page, 'nested-only.ts')).toHaveCount(0)
    } finally {
        await setAutoRevealSwitch(page, true)
    }
})
