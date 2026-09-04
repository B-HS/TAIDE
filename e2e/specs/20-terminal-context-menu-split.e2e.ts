import { KEY_CHORD } from '../lib/constants'
import { collectPaneLeaves, countTabsOfKind, readProjectLayout } from '../lib/layout'
import { expect, test } from '../lib/taide-fixture'

const SPLIT_MENU_LABEL = 'Split'
const SPLIT_RIGHT_MENU_LABEL = 'Split Right'
const TERMINAL_TAB_KIND = 'terminal'
const EXPECTED_LEAVES_AFTER_SPLIT = 2
const EXPECTED_SPLIT_DIR_FOR_RIGHT = 'horizontal'

/**
 * Coverage for `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §F: right-clicking the
 * terminal area opens the terminal's own Radix menu (`terminal-pane.tsx` wraps `TerminalView` in a
 * `ContextMenuTrigger asChild`; xterm's own right-click handler does not `preventDefault`, so the
 * event reaches the trigger), and `Split ▸ Split Right` runs `layout_open_tab_in_split` — a *new*
 * terminal tab in a new leaf to the right, not a move of the existing one. Hence the oracle counts
 * both: leaves go 1 → 2 and terminal tabs go up by exactly one.
 *
 * The directions are shown disabled (never hidden) when the pane cannot fit two `MIN_PANEL_SIZE_PX`
 * halves (`resolveSplitAvailability`), so `toBeEnabled` is asserted before the click — a disabled
 * entry would otherwise just swallow the click and the failure would surface as an opaque timeout
 * on the leaf count. Terminal mount failures are collected as `pageerror`s for the same reason
 * spec 08 does (headless WebKit WebGL — harness doc §6).
 */
test('터미널 영역 우클릭 메뉴의 Split ▸ Split Right 로 오른쪽에 새 터미널 pane 이 생긴다', async ({ page, fixtureProject }) => {
    const pageErrors: string[] = []
    page.on('pageerror', (error) => pageErrors.push(error.message))

    await page.keyboard.press(KEY_CHORD.NEW_TERMINAL)
    const terminals = page.locator('.xterm')
    try {
        await expect(terminals.first()).toBeVisible()
    } catch (mountError) {
        throw new Error(`터미널 마운트 실패 (WebGL 예외 가능성 — 하네스 문서 §6) — page errors: ${JSON.stringify(pageErrors)}`, { cause: mountError })
    }

    const layoutBeforeSplit = await readProjectLayout(page, fixtureProject.projectId)
    expect(collectPaneLeaves(layoutBeforeSplit.root)).toHaveLength(1)
    const terminalCountBeforeSplit = countTabsOfKind(layoutBeforeSplit, TERMINAL_TAB_KIND)

    await terminals.first().click({ button: 'right' })
    const menu = page.getByRole('menu')
    await expect(menu).toBeVisible()

    await menu.getByRole('menuitem', { name: SPLIT_MENU_LABEL, exact: true }).click()
    const splitRight = page.getByRole('menuitem', { name: SPLIT_RIGHT_MENU_LABEL, exact: true })
    await expect(splitRight).toBeVisible()
    await expect(splitRight).toBeEnabled()
    await splitRight.click()

    await expect(terminals).toHaveCount(EXPECTED_LEAVES_AFTER_SPLIT)
    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        expect(layout.root.node).toBe('split')
        if (layout.root.node === 'split') expect(layout.root.dir).toBe(EXPECTED_SPLIT_DIR_FOR_RIGHT)
        expect(collectPaneLeaves(layout.root)).toHaveLength(EXPECTED_LEAVES_AFTER_SPLIT)
        expect(countTabsOfKind(layout, TERMINAL_TAB_KIND)).toBe(terminalCountBeforeSplit + 1)
    }).toPass()
})
