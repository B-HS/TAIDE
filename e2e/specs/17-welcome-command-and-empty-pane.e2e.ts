import { invokeIpc } from '../lib/ipc'
import { countTabsOfKind, readProjectLayout } from '../lib/layout'
import { runPaletteCommand } from '../lib/palette'
import { openTabBarContextMenu } from '../lib/tab-bar'
import { expect, test } from '../lib/taide-fixture'

const WELCOME_TAB_TITLE = 'Welcome'
const WELCOME_COMMAND_LABEL = 'View: Welcome'
const CLOSE_ALL_TABS_MENU_LABEL = 'Close All Tabs'
const WELCOME_OPEN_FOLDER_BUTTON_LABEL = 'Open Folder'
const WELCOME_TAB_KIND = 'welcome'

/**
 * Coverage for `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §B: the `view.welcome`
 * palette command (label `"{keymap.category.view}: {app.welcome}"` → `View: Welcome`) opens a
 * Welcome tab, and a main-window pane with no tabs at all renders the Welcome screen in place of the
 * old "No file is open" caption (`pane-node-view.tsx`, gated on `settings.welcomeOnEmptyEditor`).
 *
 * A fresh fixture project starts from `default_layout()` (`layout/service.rs`), which already seeds
 * a `Welcome` tab — so the tabs are first closed through the tab bar's empty-space menu (§G, UI
 * path) to reach the zero-tab state, which is what makes the later command run observable as a tab
 * *appearing* rather than an existing one being re-activated. The Welcome *tab* and the empty-pane
 * Welcome render the same `WelcomeContainer`, so "zero tabs + Open Folder button visible" is what
 * identifies the empty-pane branch; the tab branch is then identified by the `role='tab'` row.
 */
test('View: Welcome 커맨드로 Welcome 탭이 열리고, 모든 탭을 닫으면 빈 편집 영역에 Welcome 화면이 보인다', async ({ page, fixtureProject }) => {
    const settings = await invokeIpc<{ welcomeOnEmptyEditor?: boolean }>(page, 'settings_get')
    expect(settings.welcomeOnEmptyEditor ?? true, 'precondition: settings.welcomeOnEmptyEditor must be on (default)').toBe(true)

    const welcomeTab = page.getByRole('tab', { name: WELCOME_TAB_TITLE, exact: true })
    await expect(welcomeTab).toBeVisible()

    const menu = await openTabBarContextMenu(page)
    await menu.getByRole('menuitem', { name: CLOSE_ALL_TABS_MENU_LABEL }).click()

    await expect(page.getByRole('tab')).toHaveCount(0)
    await expect(page.getByRole('button', { name: WELCOME_OPEN_FOLDER_BUTTON_LABEL })).toBeVisible()
    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        expect(countTabsOfKind(layout, WELCOME_TAB_KIND)).toBe(0)
    }).toPass()

    await runPaletteCommand(page, WELCOME_COMMAND_LABEL, { exact: true })

    await expect(welcomeTab).toBeVisible()
    await expect(welcomeTab).toHaveAttribute('aria-selected', 'true')
    await expect(page.getByRole('button', { name: WELCOME_OPEN_FOLDER_BUTTON_LABEL })).toBeVisible()
    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        expect(countTabsOfKind(layout, WELCOME_TAB_KIND)).toBe(1)
    }).toPass()
})
