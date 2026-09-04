import { EXPLORER_SELECTED_ATTRIBUTE, explorerTreeRow } from '../lib/explorer'
import { invokeIpc } from '../lib/ipc'
import { openFileViaQuickOpen } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

/**
 * Coverage for `docs/acknowledge/2026-09-04-usability-batch3-contract.md` §C (`explorerAutoReveal`,
 * VS Code `explorer.autoReveal` parity): opening a file through quick-open must make the Explorer
 * expand its ancestors and select its row — with no click on the tree at all — and switching the
 * active tab must move that selection. `src/nested-only.ts` sits in a directory a fresh fixture
 * project has never expanded, so its row can only appear through the auto-reveal path
 * (`use-explorer-auto-reveal.ts` → `tree_reveal`); `index.ts` is a root-level row that is already
 * visible, exercising the IPC-free `select-only` branch of `decideAutoReveal`.
 *
 * The setting is not reset by the harness before a run (only `globalTeardown` restores settings),
 * so the precondition read below turns "the developer had auto-reveal switched off" into an
 * explicit failure instead of a confusing timeout on the first row assertion.
 */
test('퀵오픈으로 파일을 열면 파일 트리가 자동으로 펼쳐져 그 행을 선택하고, 탭을 바꾸면 선택이 옮겨진다', async ({ page, fixtureProject }) => {
    void fixtureProject

    const settings = await invokeIpc<{ explorerAutoReveal?: boolean }>(page, 'settings_get')
    expect(settings.explorerAutoReveal ?? true, 'precondition: settings.explorerAutoReveal must be on (default)').toBe(true)

    await expect(page.getByRole('tree')).toBeVisible()

    await openFileViaQuickOpen(page, 'nested-only')
    await expect(page.getByRole('tab', { name: /nested-only\.ts/ })).toBeVisible()

    const nestedRow = explorerTreeRow(page, 'nested-only.ts')
    await expect(nestedRow).toBeVisible()
    await expect(nestedRow).toHaveAttribute(EXPLORER_SELECTED_ATTRIBUTE, 'true')

    await openFileViaQuickOpen(page, 'index.ts')
    await expect(page.getByRole('tab', { name: /index\.ts/ })).toBeVisible()

    const indexRow = explorerTreeRow(page, 'index.ts')
    await expect(indexRow).toHaveAttribute(EXPLORER_SELECTED_ATTRIBUTE, 'true')
    await expect(nestedRow).toHaveAttribute(EXPLORER_SELECTED_ATTRIBUTE, 'false')
})
