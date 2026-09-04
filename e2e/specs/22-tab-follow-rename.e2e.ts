import { access, readFile } from 'node:fs/promises'
import path from 'node:path'
import { explorerInlineNameInput, explorerTreeRow } from '../lib/explorer'
import { collectPaneTabs, readProjectLayout } from '../lib/layout'
import { openFileViaQuickOpen } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

const RENAME_MENU_LABEL = 'Rename'
const ORIGINAL_FILE_NAME = 'other.ts'
const RENAMED_FILE_NAME = 'renamed-by-e2e.ts'
const ORIGINAL_FILE_MARKER = 'export const shout'

/**
 * Coverage for audit §4-B A3 (open tabs follow a rename) together with
 * `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §G.2-4 (the tab menu's own `Rename`
 * entry): renaming the file behind an open tab must repoint that tab instead of leaving it on a
 * path that no longer exists. Rust rewrites both `kind` and `title`
 * (`layout/service.rs`, `retarget_file_tabs`), so the tab's displayed name is the visible half of
 * the same contract and is what this spec asserts on — `lib/layout.ts`'s `PaneTab` deliberately
 * models `kind` as `{ kind: string }` only, so the IPC oracle checks titles too.
 *
 * The rename is started from the *tab's* context menu, which hands the request to the Explorer over
 * `explorer-rename-bridge.ts` (view switch → `tree_reveal` → `startRename`). That path is also the
 * one that mounts the inline row *after* an await, so the row is not competing with the closing
 * menu's focus restore — it cancels on blur (`file-tree-draft-row.tsx`), hence `fill`/`press` only.
 */
test('열린 탭의 파일을 탐색기에서 이름 바꾸면 탭 제목이 새 이름을 따라간다', async ({ page, fixtureProject }) => {
    await openFileViaQuickOpen(page, ORIGINAL_FILE_NAME)
    const originalTab = page.getByRole('tab', { name: /other\.ts/ })
    await expect(originalTab).toBeVisible()

    await originalTab.click({ button: 'right' })
    const tabMenu = page.getByRole('menu')
    await expect(tabMenu).toBeVisible()
    await tabMenu.getByRole('menuitem', { name: RENAME_MENU_LABEL, exact: true }).click()

    const nameInput = explorerInlineNameInput(page)
    await expect(nameInput).toBeVisible()
    await nameInput.fill(RENAMED_FILE_NAME)
    await nameInput.press('Enter')

    await expect(page.getByRole('tab', { name: /renamed-by-e2e\.ts/ })).toBeVisible()
    await expect(originalTab).toHaveCount(0)
    await expect(explorerTreeRow(page, RENAMED_FILE_NAME)).toBeVisible()

    await expect(async () => {
        const layout = await readProjectLayout(page, fixtureProject.projectId)
        const titles = collectPaneTabs(layout).map((tab) => tab.title)
        expect(titles).toContain(RENAMED_FILE_NAME)
        expect(titles).not.toContain(ORIGINAL_FILE_NAME)
    }).toPass()

    const renamedFilePath = path.join(fixtureProject.rootDir, RENAMED_FILE_NAME)
    expect(await readFile(renamedFilePath, 'utf8')).toContain(ORIGINAL_FILE_MARKER)

    const originalFileStillOnDisk = await access(path.join(fixtureProject.rootDir, ORIGINAL_FILE_NAME)).then(
        () => true,
        () => false,
    )
    expect(originalFileStillOnDisk).toBe(false)
})
