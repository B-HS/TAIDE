import { collectPaneTabs, countTabsOfKind, readProjectLayout } from '../lib/layout'
import { openFileViaQuickOpen } from '../lib/palette'
import { openTabBarContextMenu } from '../lib/tab-bar'
import { expect, test } from '../lib/taide-fixture'

const PIN_MENU_LABEL = 'Pin'
const CLOSE_ALL_TABS_MENU_LABEL = 'Close All Tabs'
const PINNED_FILE_NAME = 'index.ts'
const UNPIN_BUTTON_LABEL = 'Unpin index.ts'
const CLOSE_BUTTON_LABEL = 'Close index.ts'
const FILE_TAB_KIND = 'file'
const EXPECTED_SURVIVING_TABS = 1

/**
 * Coverage for audit §4-B B4 (pinned-tab protection, `docs/features/tabs.md` §3): a pinned tab must
 * survive every bulk close (`pane-tab-bar.tsx`'s `handleCloseAll`/`CloseOthers`/`CloseToRight`/
 * `CloseSaved` all `continue` on `tab.pinned`), and the tab's own trailing icon button must
 * **unpin** rather than close — that button already rendered a pin glyph and announced itself as
 * "Unpin …" while still running `onClose`, so one click on what looked like the unpin affordance
 * discarded the tab (`tab-item.tsx`).
 *
 * The two halves are asserted in order, with the final `Close All Tabs` acting as the positive
 * control: it proves the same menu entry does close this tab once it is unpinned, so the earlier
 * survival is the pin guard and not a menu item that silently did nothing.
 */
test('고정한 탭은 모든 탭 닫기에도 살아남고, 닫기 버튼은 닫는 대신 고정을 해제한다', async ({ page, fixtureProject }) => {
    await openFileViaQuickOpen(page, PINNED_FILE_NAME)
    const fileTab = page.getByRole('tab', { name: /index\.ts/ })
    await expect(fileTab).toBeVisible()

    await fileTab.click({ button: 'right' })
    const tabMenu = page.getByRole('menu')
    await expect(tabMenu).toBeVisible()
    await tabMenu.getByRole('menuitem', { name: PIN_MENU_LABEL, exact: true }).click()

    const unpinButton = page.getByRole('button', { name: UNPIN_BUTTON_LABEL, exact: true })
    await expect(unpinButton).toBeVisible()

    const menu = await openTabBarContextMenu(page)
    await menu.getByRole('menuitem', { name: CLOSE_ALL_TABS_MENU_LABEL }).click()

    await expect(page.getByRole('tab')).toHaveCount(EXPECTED_SURVIVING_TABS)
    await expect(fileTab).toBeVisible()
    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        expect(collectPaneTabs(layout)).toHaveLength(EXPECTED_SURVIVING_TABS)
        expect(countTabsOfKind(layout, FILE_TAB_KIND)).toBe(EXPECTED_SURVIVING_TABS)
    }).toPass()

    await unpinButton.click()

    await expect(page.getByRole('tab')).toHaveCount(EXPECTED_SURVIVING_TABS)
    await expect(page.getByRole('button', { name: CLOSE_BUTTON_LABEL, exact: true })).toBeVisible()

    const menuAfterUnpin = await openTabBarContextMenu(page)
    await menuAfterUnpin.getByRole('menuitem', { name: CLOSE_ALL_TABS_MENU_LABEL }).click()
    await expect(page.getByRole('tab')).toHaveCount(0)
})
