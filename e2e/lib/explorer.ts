import type { Page } from '@playwright/test'
import { escapeRegExpLiteral } from './palette'

/**
 * Locates one Explorer file-tree row by its displayed file name, scoped to the `role='tree'`
 * container (`file-tree.tsx`). Rows are `role='treeitem'` (`file-tree-row.tsx`), and the accessible
 * name is the row's text content — the file name plus, when the file has a git status, a badge
 * (`U`/`M`/…), so the name is matched as a prefix rather than exactly.
 */
export const explorerTreeRow = (page: Page, fileName: string) =>
    page.getByRole('tree').getByRole('treeitem', { name: new RegExp(`^${escapeRegExpLiteral(fileName)}`) })

/**
 * Selection state is `aria-selected` on the row (`file-tree-row.tsx`); the highlight class
 * (`bg-explorer-item-selected`/`bg-explorer-item-focused`) is derived from the same flag, so the
 * attribute is the single stable oracle.
 */
export const EXPLORER_SELECTED_ATTRIBUTE = 'aria-selected'

/**
 * The Explorer's inline name editor — the one `input` the tree ever renders
 * (`file-tree-draft-row.tsx`), shared by the create draft row and the rename row.
 *
 * It commits on `Enter` and **cancels on blur**, so a spec must reach it with `fill`/`press` only:
 * clicking anything else first (including elsewhere in the tree) discards the edit before the
 * assertion runs. A rejected name keeps the input mounted with `aria-invalid='true'` and shows the
 * reason in a `role='tooltip'`.
 */
export const explorerInlineNameInput = (page: Page) => page.getByRole('tree').locator('input').first()
