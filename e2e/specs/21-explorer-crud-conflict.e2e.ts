import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { explorerInlineNameInput, explorerTreeRow } from '../lib/explorer'
import { expect, test } from '../lib/taide-fixture'

const NEW_FILE_BUTTON_LABEL = 'New File'
const EXISTING_FILE_NAME = 'index.ts'
const EXISTING_FILE_MARKER = 'export const greet'
const CONFLICT_FREE_FILE_NAME = 'crud-conflict-free.ts'
const DUPLICATE_ERROR_TEXT = 'already exists at this location'
const INVALID_ATTRIBUTE = 'aria-invalid'

/**
 * Coverage for the Explorer's create-entry conflict guard (`use-explorer-entry-crud.ts` →
 * `validateEntryName`): a new entry whose name collides with an existing sibling must be refused
 * *before* any IPC, keeping the inline row open with the reason attached, and the file already on
 * disk must be untouched — `file_create` uses `create_new` (`domain/file/service.rs`), so a guard
 * regression that let the call through would surface as an error toast, never as data loss, which
 * is exactly why the disk read below is the assertion that matters.
 *
 * The draft is started from the Explorer toolbar's `New File` button rather than the tree's context
 * menu because the toolbar mounts the inline row synchronously with nothing else competing for
 * focus: the row commits on `Enter` and **cancels on blur** (`file-tree-draft-row.tsx`), so the
 * whole test touches it with `fill`/`press` only. With no row selected, the draft's parent is the
 * project root (`targetDirFor(null)`), which is where the colliding `index.ts` lives.
 */
test('탐색기에서 이미 있는 이름으로 파일을 만들면 인라인 오류로 막히고, 이름을 바꾸면 그대로 생성된다', async ({ page, fixtureProject }) => {
    await expect(page.getByRole('tree')).toBeVisible()
    await page.getByRole('button', { name: NEW_FILE_BUTTON_LABEL, exact: true }).click()

    const nameInput = explorerInlineNameInput(page)
    await expect(nameInput).toBeVisible()

    await nameInput.fill(EXISTING_FILE_NAME)
    await nameInput.press('Enter')

    await expect(nameInput).toHaveAttribute(INVALID_ATTRIBUTE, 'true')
    await expect(page.getByRole('tooltip').filter({ hasText: DUPLICATE_ERROR_TEXT })).toBeVisible()
    await expect(explorerTreeRow(page, EXISTING_FILE_NAME)).toHaveCount(1)

    const existingFilePath = path.join(fixtureProject.rootDir, EXISTING_FILE_NAME)
    expect(await readFile(existingFilePath, 'utf8')).toContain(EXISTING_FILE_MARKER)

    await nameInput.fill(CONFLICT_FREE_FILE_NAME)
    await nameInput.press('Enter')

    await expect(nameInput).toHaveCount(0)
    await expect(explorerTreeRow(page, CONFLICT_FREE_FILE_NAME)).toBeVisible()
    await expect(page.getByRole('tab', { name: /crud-conflict-free\.ts/ })).toBeVisible()

    const createdFilePath = path.join(fixtureProject.rootDir, CONFLICT_FREE_FILE_NAME)
    await expect(async () => {
        expect(await readFile(createdFilePath, 'utf8')).toBe('')
    }).toPass()
})
