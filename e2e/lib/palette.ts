import type { Page } from '@playwright/test'
import { KEY_CHORD, PALETTE_MODE_PREFIX } from './constants'

/**
 * Opens the command palette in quick-open (file search) mode via `Meta+P` and returns its text
 * input, scoped to the palette's dialog rather than matched globally — cmdk (the palette's
 * underlying primitive) doesn't expose a stable `data-testid`, so every palette interaction in this
 * harness goes through this one locator instead of guessing at cmdk's internal ARIA roles.
 */
export const openPalette = async (page: Page) => {
    await page.keyboard.press(KEY_CHORD.QUICK_OPEN)
    const dialog = page.getByRole('dialog')
    await dialog.waitFor({ state: 'visible' })
    return dialog.locator('input').first()
}

export const openFileViaQuickOpen = async (page: Page, fileNameQuery: string) => {
    const input = await openPalette(page)
    await input.fill(fileNameQuery)
    const option = page.getByRole('option').filter({ hasText: fileNameQuery }).first()
    await option.waitFor({ state: 'visible' })
    await option.click()
}

export const escapeRegExpLiteral = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

/**
 * Runs an app command by title through the palette's `>` command mode (opened directly via
 * `Meta+Shift+P`, which seeds the `>` prefix — see `command-palette.tsx`'s `useGlobalKeymap`).
 *
 * `titleQuery` matches by case-insensitive substring by default, same as `hasText` — pass
 * `{ exact: true }` with the full `"{category}: {title}"` label (see `formatCategorizedLabel` in
 * `command-registry.ts`) when a shorter query would also match a different command's label.
 */
export const runPaletteCommand = async (page: Page, titleQuery: string, matchOptions?: { exact?: boolean }) => {
    await page.keyboard.press(KEY_CHORD.COMMAND_PALETTE)
    const dialog = page.getByRole('dialog')
    await dialog.waitFor({ state: 'visible' })
    const input = dialog.locator('input').first()
    await input.fill(`${PALETTE_MODE_PREFIX.COMMAND}${titleQuery}`)
    const option = matchOptions?.exact
        ? page
              .getByRole('option')
              .filter({ hasText: new RegExp(`^${escapeRegExpLiteral(titleQuery)}$`) })
              .first()
        : page.getByRole('option').filter({ hasText: titleQuery }).first()
    await option.waitFor({ state: 'visible' })
    await option.click()
}
