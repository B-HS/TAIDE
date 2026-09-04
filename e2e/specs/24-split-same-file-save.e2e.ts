import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { KEY_CHORD } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { collectPaneLeaves, collectPaneTabs, readProjectLayout } from '../lib/layout'
import { openFileViaQuickOpen } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

const SPLIT_MENU_LABEL = 'Split'
const SPLIT_RIGHT_MENU_LABEL = 'Split Right'
const SHARED_FILE_NAME = 'index.ts'
const WELCOME_TAB_NAME = /Welcome/
const SAVE_MARKER_LINE = "export const e2eSplitMarker = 'taide-e2e-split-same-file-save'"
const DIRTY_DOT_SELECTOR = '.bg-tab-bar-dirty-dot'
const EXPECTED_PANE_LEAVES = 2
const EXPECTED_OPEN_COPIES = 2

type OpenedFile = { path: string; content: string }

/**
 * Coverage for d-51 F1 (`docs/features/editor.md` §저장 정착): save settling is keyed by **path, not
 * pane**. With the same file open in two panes both tabs share one Monaco model
 * (`entities/editor/model-registry.ts` keys models by path), so an edit in either pane dirties both
 * tabs — and the disk write must then settle both through `file-save-settle-registry.ts`. Before
 * that broadcast existed, the pane that did not run the save kept its dirty dot, resurrected its
 * hot-exit mirror and raised a bogus "changed on disk" banner, so the two dirty-dot counts here
 * (2 → 0) are the whole regression.
 *
 * Getting one file into two panes is done entirely through the UI: quick-open, the tab menu's
 * `Split ▸ Split Right` (which *moves* the tab into a new leaf — `layout_split`), then re-focusing
 * the original pane by clicking its `Welcome` tab and quick-opening the same file again. The second
 * open lands as a separate tab because `open_tab`'s kind de-duplication only scans the target
 * leaf's own tabs (`layout/service.rs`, contract §3.4-7).
 */
test('같은 파일이 두 pane 에 열려 있으면 한쪽에서 저장해도 양쪽 탭의 dirty 표시가 함께 사라진다', async ({ page, fixtureProject }) => {
    await openFileViaQuickOpen(page, SHARED_FILE_NAME)
    const fileTabs = page.getByRole('tab', { name: /index\.ts/ })
    await expect(fileTabs).toHaveCount(1)

    await fileTabs.first().click({ button: 'right' })
    const tabMenu = page.getByRole('menu')
    await expect(tabMenu).toBeVisible()
    await tabMenu.getByRole('menuitem', { name: SPLIT_MENU_LABEL, exact: true }).click()
    const splitRight = page.getByRole('menuitem', { name: SPLIT_RIGHT_MENU_LABEL, exact: true })
    await expect(splitRight).toBeVisible()
    await splitRight.click()

    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        expect(collectPaneLeaves(layout.root)).toHaveLength(EXPECTED_PANE_LEAVES)
    }).toPass()

    await page.getByRole('tab', { name: WELCOME_TAB_NAME }).click()
    await openFileViaQuickOpen(page, SHARED_FILE_NAME)

    await expect(fileTabs).toHaveCount(EXPECTED_OPEN_COPIES)
    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        expect(collectPaneTabs(layout).filter((tab) => tab.title === SHARED_FILE_NAME)).toHaveLength(EXPECTED_OPEN_COPIES)
    }).toPass()

    const editor = page.locator('.monaco-editor').first()
    await editor.click()
    await page.keyboard.press('Meta+End')
    await page.evaluate((line) => navigator.clipboard.writeText(`\n${line}`), SAVE_MARKER_LINE)
    await page.keyboard.press(KEY_CHORD.PASTE)

    await expect(fileTabs.locator(DIRTY_DOT_SELECTOR)).toHaveCount(EXPECTED_OPEN_COPIES)

    await page.keyboard.press(KEY_CHORD.SAVE)
    await expect(fileTabs.locator(DIRTY_DOT_SELECTOR)).toHaveCount(0)

    const filePath = path.join(fixtureProject.rootDir, SHARED_FILE_NAME)
    await expect(async () => {
        expect(await readFile(filePath, 'utf8')).toContain(SAVE_MARKER_LINE)
    }).toPass()

    const reopened = await invokeIpc<OpenedFile>(page, 'file_open', { path: filePath })
    expect(reopened.content).toContain(SAVE_MARKER_LINE)
})
