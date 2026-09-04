import type { Page } from '@playwright/test'

const CONTEXT_MENU_TRIGGER_SELECTOR = '[data-slot="context-menu-trigger"]'

/**
 * The tab bar's empty-space filler (`pane-tab-bar.tsx`, the `min-w-8 flex-1` div after the tabs).
 * It carries the empty-space context menu's trigger (`TabBarContextMenu` → Radix `Trigger asChild`,
 * which stamps `data-slot='context-menu-trigger'` onto the child) and is a *direct* child of the
 * `role='tablist'` scroller — every tab's own `TabContextMenu` trigger sits one wrapper deeper, so
 * the `:scope >` restriction is what keeps this from matching a tab.
 */
export const tabBarFiller = (page: Page) => page.getByRole('tablist').locator(`:scope > ${CONTEXT_MENU_TRIGGER_SELECTOR}`).first()

/** Right-clicks the filler and returns the opened Radix menu (`role='menu'`). */
export const openTabBarContextMenu = async (page: Page) => {
    await tabBarFiller(page).click({ button: 'right' })
    const menu = page.getByRole('menu')
    await menu.waitFor({ state: 'visible' })
    return menu
}
