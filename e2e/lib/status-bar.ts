import type { Page } from '@playwright/test'

const CURSOR_POSITION_PATTERN = /^Ln \d+, Col \d+$/

/** Locates the status bar's `Ln N, Col M` cursor-position label (`editor.cursorPosition`, pinned to English via `DEFAULT_TEST_LOCALE`). */
export const cursorPositionLabel = (page: Page) => page.getByText(CURSOR_POSITION_PATTERN)
