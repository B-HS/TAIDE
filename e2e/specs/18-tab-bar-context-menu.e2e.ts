import { countTabsOfKind, readProjectLayout } from '../lib/layout'
import { openTabBarContextMenu } from '../lib/tab-bar'
import { expect, test } from '../lib/taide-fixture'

const NEW_TERMINAL_MENU_LABEL = 'New Terminal'
const SPLIT_MENU_LABEL = 'Split'
const CLOSE_ALL_TABS_MENU_LABEL = 'Close All Tabs'
const TERMINAL_TAB_TITLE = 'Terminal'
const TERMINAL_TAB_KIND = 'terminal'

/**
 * Coverage for `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §G: right-clicking the
 * tab bar's empty space opens a menu of its own (`tab-bar-context-menu.tsx`), whose "New Terminal"
 * entry creates a terminal tab in that pane, and whose entries follow the tab menu's *hide, do not
 * disable* policy (`tabs.md` §3.1, `buildTabBarMenuItems`): with no active tab there is nothing
 * `layout_split` could move, so the `Split` submenu must be absent — not greyed — while the creation
 * entries stay.
 *
 * Counts are relative to whatever `default_layout()` seeded (currently one `Terminal` tab), so the
 * spec does not break if the default tab set changes. The "with tabs" pass doubles as the positive
 * control for the `Split` assertion: it proves the entry does render when its precondition holds,
 * so its later absence is the hiding rule and not a locator that never matches.
 */
test('탭 바 여백 우클릭 메뉴의 New Terminal 로 터미널 탭이 생기고, 탭이 0개면 Split 항목이 숨겨진다', async ({ page, fixtureProject }) => {
    const initialLayout = await readProjectLayout(page, fixtureProject.projectId)
    const initialTerminalCount = countTabsOfKind(initialLayout, TERMINAL_TAB_KIND)
    const terminalTabs = page.getByRole('tab', { name: TERMINAL_TAB_TITLE, exact: true })
    await expect(terminalTabs).toHaveCount(initialTerminalCount)

    const menuWithTabs = await openTabBarContextMenu(page)
    await expect(menuWithTabs.getByRole('menuitem', { name: SPLIT_MENU_LABEL, exact: true })).toBeVisible()
    await menuWithTabs.getByRole('menuitem', { name: NEW_TERMINAL_MENU_LABEL }).click()

    await expect(terminalTabs).toHaveCount(initialTerminalCount + 1)
    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        expect(countTabsOfKind(layout, TERMINAL_TAB_KIND)).toBe(initialTerminalCount + 1)
    }).toPass()

    const menuBeforeClose = await openTabBarContextMenu(page)
    await menuBeforeClose.getByRole('menuitem', { name: CLOSE_ALL_TABS_MENU_LABEL }).click()
    await expect(page.getByRole('tab')).toHaveCount(0)

    const menuWithoutTabs = await openTabBarContextMenu(page)
    try {
        await expect(menuWithoutTabs.getByRole('menuitem', { name: NEW_TERMINAL_MENU_LABEL })).toBeVisible()
        await expect(menuWithoutTabs.getByRole('menuitem', { name: SPLIT_MENU_LABEL, exact: true })).toHaveCount(0)
        await expect(menuWithoutTabs.getByRole('menuitem', { name: CLOSE_ALL_TABS_MENU_LABEL })).toHaveCount(0)
    } finally {
        await page.keyboard.press('Escape')
    }
    await expect(menuWithoutTabs).toBeHidden()
})
