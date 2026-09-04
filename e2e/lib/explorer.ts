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
