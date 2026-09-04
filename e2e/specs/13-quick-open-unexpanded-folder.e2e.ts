import { openFileViaQuickOpen } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

/**
 * Regression coverage for d-42 item d (`docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md`
 * §3): before the fix, the command palette's file quick-open mode fuzzy-filtered the Explorer tree's
 * lazily-loaded `tree_rows` cache, so it could only ever surface a file inside a directory the tree
 * had already been expanded into — a fixture project starts with every directory collapsed. This
 * spec never expands `src/` itself (no click, no `tree_toggle` call), so a pass can only mean
 * quick-open found `src/nested-only.ts` through the project-wide `search_list_files` index
 * (`projectFilesQueryOptions`), not through the tree. The tree does end up expanded once the tab is
 * open — `explorerAutoReveal` defaults on and reveals the active file — but that runs *after* the
 * open and through `tree_reveal`, so it cannot be what surfaced the file in the palette.
 */
test('트리를 한 번도 확장하지 않아도 퀵오픈이 미확장 폴더의 파일을 찾는다', async ({ page, fixtureProject }) => {
    void fixtureProject

    await openFileViaQuickOpen(page, 'nested-only')

    await expect(page.getByRole('tab', { name: /nested-only\.ts/ })).toBeVisible()
    await expect(page.locator('.monaco-editor').first()).toContainText('nestedOnly')
})
