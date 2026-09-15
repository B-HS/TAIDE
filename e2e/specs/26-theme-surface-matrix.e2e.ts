import { appendFile } from 'node:fs/promises'
import path from 'node:path'
import type { Locator, Page } from '@playwright/test'
import { GIT_STATUS_SETTLE_TIMEOUT_MS, KEY_CHORD, PALETTE_MODE_PREFIX } from '../lib/constants'
import { EXPLORER_SELECTED_ATTRIBUTE, explorerTreeRow } from '../lib/explorer'
import { invokeIpc } from '../lib/ipc'
import { openFileViaQuickOpen, runPaletteCommand } from '../lib/palette'
import { E2E_ROOT_DIR } from '../lib/paths'
import { openTabBarContextMenu } from '../lib/tab-bar'
import { expect, test } from '../lib/taide-fixture'

const MINUTE_MS = 60_000
const SETUP_TIME_BUDGET_MS = 3 * MINUTE_MS
const THEME_TIME_BUDGET_MS = 90_000
const MATRIX_TIME_BUDGET_MS = 40 * MINUTE_MS
const SPEC_TIMEOUT_MS = SETUP_TIME_BUDGET_MS + MATRIX_TIME_BUDGET_MS
const THEME_APPLY_TIMEOUT_MS = 15_000
const OVERLAY_SETTLE_TIMEOUT_MS = 10_000
const MOUSE_DRAG_STEPS = 12

const ARTIFACT_DIR = path.join(E2E_ROOT_DIR, 'artifacts', 'theme-matrix')
const ARTIFACT_NAME_UNSAFE_PATTERN = /[^A-Za-z0-9._-]+/g

const SETTINGS_COMMAND_LABEL = 'App: Settings'
const KEYBINDINGS_COMMAND_TITLE = 'Edit All Shortcuts'
const SETTINGS_TAB_TITLE = 'Settings'
const TERMINAL_TAB_TITLE = 'Terminal'
const SIDEBAR_SWITCH_LABEL = 'Switch sidebar view'
const SIDEBAR_VIEW_LABEL = { FILES: 'Explorer', GIT: 'Git' } as const
const SWITCH_STATE = { ON: 'checked', OFF: 'unchecked' } as const

const EDITOR_FILE_NAME = 'index.ts'
const EDITOR_TAB_NAME_PATTERN = /index\.ts/
const HOVERED_TREE_FILE_NAME = 'other.ts'
const DIRTY_FILE_NAME = 'other.ts'
const GIT_DIRTY_MARKER_LINE = '\nexport const e2eThemeMatrixMarker = true\n'
const PALETTE_SAMPLE_QUERY = 'view'

const EDITOR_BACKGROUND_TOKEN = 'editor.background'
const EDITOR_BACKGROUND_VAR = '--taide-editor-background'
const ROOT_ELEMENT_SELECTOR = 'html'
const THEME_TYPE_ATTRIBUTE = 'data-theme-type'
const SELECTED_ATTRIBUTE = 'aria-selected'
const PRESSED_ATTRIBUTE = 'aria-pressed'

const DOCUMENT_START_CHORD = 'Meta+Home'
const EXTEND_SELECTION_KEY = 'Shift+ArrowRight'
const EDITOR_FIND_CHORD = 'Meta+F'
const ESCAPE_KEY = 'Escape'

const MONACO_EDITOR_SELECTOR = '.monaco-editor'
const MONACO_CURRENT_LINE_SELECTOR = '.current-line'
const MONACO_SELECTION_SELECTOR = '.selected-text'
const MONACO_FIND_WIDGET_SELECTOR = '.find-widget.visible'
const MONACO_FIND_MATCH_SELECTOR = '.findMatch, .currentFindMatch'
const XTERM_SELECTOR = '.xterm'
const THEME_BUTTON_SELECTOR = `button[${PRESSED_ATTRIBUTE}]`
const SIDEBAR_PANEL_SELECTOR = 'div.bg-explorer-background:has([role="tablist"])'
const DIALOG_OVERLAY_SELECTOR = '[data-slot="dialog-overlay"]'
const TOOLTIP_CONTENT_SELECTOR = '[data-slot="tooltip-content"]'
const GIT_SECTION_HEADER_SELECTOR = '[data-git-section-header]'
const CHANGES_SECTION_TITLE_PATTERN = /^Changes/

const EDITOR_DRAG_START_OFFSET = { x: 80, y: 12 }
const EDITOR_DRAG_END_OFFSET = { x: 280, y: 50 }
const TERMINAL_DRAG_START_OFFSET = { x: 8, y: 8 }
const TERMINAL_DRAG_END_OFFSET = { x: 320, y: 36 }
const POINTER_PARK_POINT = { x: 0, y: 0 }

const THEME_SWITCH_STEP = 'theme-switch'
const MATRIX_BUDGET_STEP = 'matrix-time-budget'
const THEME_BUDGET_REASON = `테마당 시간 상한(${THEME_TIME_BUDGET_MS}ms)을 넘겨 이 표면을 건너뛰었습니다.`
const MATRIX_BUDGET_REASON = `매트릭스 전체 시간 상한(${MATRIX_TIME_BUDGET_MS}ms)을 넘겨 이 테마를 통째로 건너뛰었습니다.`
const MISSING_BOUNDING_BOX_MESSAGE = '드래그 대상의 bounding box 를 읽지 못했습니다 — 요소가 화면에 없습니다.'
const FOLLOW_SYSTEM_THEME_PRECONDITION_MESSAGE =
    'precondition: settings.followSystemTheme must be off — 켜져 있으면 theme_get_current 가 선택한 테마 대신 시스템 테마를 돌려주므로 매트릭스가 전부 같은 두 테마로 찍힌다.'
const BUNDLED_THEME_PRECONDITION_MESSAGE = 'precondition: theme_list 가 builtin 테마를 최소 1종 돌려줘야 한다.'
const MATRIX_FAILURE_MESSAGE =
    '표면별 캡처 실패 목록 — 실패한 항목만 누락이고 나머지 스크린샷은 e2e/artifacts/theme-matrix/ 아래에 그대로 남아 있다(비전 검토는 남은 것으로 진행 가능).'

type BundledThemeSummary = { id: string; name: string; type: 'dark' | 'light'; builtin: boolean }
type ResolvedThemeColors = { colors: Record<string, string> }
type MatrixSettings = { themeId?: string; followSystemTheme?: boolean }
type ScreenPoint = { x: number; y: number }
type SurfaceRecorder = (target?: Locator) => Promise<void>
type ThemeSurface = { key: string; record: (page: Page, save: SurfaceRecorder) => Promise<void> }
type MatrixFailure = { themeId: string; surface: string; reason: string }

const toArtifactDirName = (themeId: string) => themeId.replace(ARTIFACT_NAME_UNSAFE_PATTERN, '-')

const describeFailure = (error: unknown) => (error instanceof Error ? error.message : String(error))

const readCssVariable = (page: Page, variableName: string) =>
    page.evaluate((name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim(), variableName)

const sidebarPanel = (page: Page) => page.locator(SIDEBAR_PANEL_SELECTOR).first()

const sidebarViewTab = (page: Page, label: string) =>
    page.getByRole('tablist', { name: SIDEBAR_SWITCH_LABEL }).getByRole('tab', { name: label, exact: true })

const showSidebarView = async (page: Page, label: string) => {
    const viewTab = sidebarViewTab(page, label)
    await viewTab.click()
    await expect(viewTab).toHaveAttribute(SELECTED_ATTRIBUTE, 'true')
}

/**
 * Editor tabs are matched by substring, never `exact`: a tab's accessible name is computed from its
 * contents, and `tab-item.tsx` puts a close/unpin `IconButton` inside every tab whose own
 * `aria-label` ("Close Terminal") joins that name. `.first()` then picks one of the two identically
 * titled Terminal tabs the setup phase leaves open.
 */
const settingsTab = (page: Page) => page.getByRole('tab', { name: SETTINGS_TAB_TITLE }).first()

const editorTab = (page: Page) => page.getByRole('tab', { name: EDITOR_TAB_NAME_PATTERN }).first()

const terminalTab = (page: Page) => page.getByRole('tab', { name: TERMINAL_TAB_TITLE }).first()

/**
 * The editor tab bar, told apart from the sidebar's own `role='tablist'` (the Explorer/Search/Git/
 * Outline switcher, `explorer-panel.tsx`) by the Settings tab the setup phase always leaves open in
 * it — the pane tab bar carries no accessible name of its own to select on.
 */
const editorTabBar = (page: Page) =>
    page
        .getByRole('tablist')
        .filter({ has: page.getByRole('tab', { name: SETTINGS_TAB_TITLE }) })
        .first()

const activateTab = async (tab: Locator) => {
    await tab.click()
    await expect(tab).toHaveAttribute(SELECTED_ATTRIBUTE, 'true')
}

/**
 * One theme's button in the Settings theme picker (`theme-picker.tsx`), matched through the name
 * span rather than `getByRole('button', { name })`. Role-name matching is substring-based by
 * default, so once sibling variants land (§1.D: `GitHub Dark` alongside `GitHub Dark Dimmed`,
 * `Tokyo Night` alongside `Tokyo Night Storm`) a name query would resolve two buttons and trip
 * strict mode; an exact text match on the name span cannot. `.first()` keeps a user's own custom
 * theme that happens to reuse a bundled name out of the way — the picker renders built-in and
 * bundled sections before the custom one.
 */
const themeButton = (page: Page, themeName: string) =>
    page
        .locator(THEME_BUTTON_SELECTOR)
        .filter({ has: page.getByText(themeName, { exact: true }) })
        .first()

const switchRowSelector = (state: string) => `label:has([data-slot="switch"][data-state="${state}"])`

const focusEditor = async (page: Page) => {
    await activateTab(editorTab(page))
    const editor = page.locator(MONACO_EDITOR_SELECTOR).first()
    await expect(editor).toBeVisible()
    await editor.click()
    return editor
}

const dragWithin = async (page: Page, target: Locator, from: ScreenPoint, to: ScreenPoint) => {
    const box = await target.boundingBox()
    if (!box) throw new Error(MISSING_BOUNDING_BOX_MESSAGE)
    await page.mouse.move(box.x + from.x, box.y + from.y)
    await page.mouse.down()
    await page.mouse.move(box.x + to.x, box.y + to.y, { steps: MOUSE_DRAG_STEPS })
    await page.mouse.up()
}

const captureSurface = async (page: Page, themeId: string, surfaceKey: string, target?: Locator) => {
    const file = path.join(ARTIFACT_DIR, toArtifactDirName(themeId), `${surfaceKey}.png`)
    if (target) {
        await target.screenshot({ path: file })
        return
    }
    await page.screenshot({ path: file })
}

/**
 * Switches the app to `theme` through the Settings picker and only returns once the new palette is
 * actually painted, so a screenshot can never be filed under a theme it does not show.
 *
 * Two oracles, both DOM: `aria-pressed` is derived from `settings.themeId`, which proves the click
 * reached the backend; the `--taide-editor-background` comparison proves `ThemeProvider`'s
 * `applyThemeVariables` has since rewritten the custom properties on `<html>` with *this* theme's
 * colors (it writes the resolved values verbatim — `theme-variables.ts`). The expected value comes
 * from a `theme_get` read, which the § 조작은 UI, 검증만 IPC rule allows: it is a read used to build
 * an assertion, never a state change.
 */
const applyTheme = async (page: Page, theme: BundledThemeSummary) => {
    await activateTab(settingsTab(page))
    const button = themeButton(page, theme.name)
    await button.scrollIntoViewIfNeeded()
    await button.click()
    await expect(button).toHaveAttribute(PRESSED_ATTRIBUTE, 'true', { timeout: THEME_APPLY_TIMEOUT_MS })

    const resolved = await invokeIpc<ResolvedThemeColors>(page, 'theme_get', { themeId: theme.id })
    const expectedEditorBackground = resolved.colors[EDITOR_BACKGROUND_TOKEN]
    await expect(async () => {
        expect(await readCssVariable(page, EDITOR_BACKGROUND_VAR)).toBe(expectedEditorBackground)
    }).toPass({ timeout: THEME_APPLY_TIMEOUT_MS })
    await expect(page.locator(ROOT_ELEMENT_SELECTOR)).toHaveAttribute(THEME_TYPE_ATTRIBUTE, theme.type)
}

/**
 * Every surface the vision review needs (contract d-61 §1.E). Each entry re-establishes its own
 * preconditions instead of inheriting the previous one's state, so one failing surface costs one
 * screenshot rather than the rest of that theme's row.
 *
 * Contained regions (sidebar, editor, terminal, tab bar, one settings row) are captured as element
 * screenshots — tight crops keep the review focused and the artifact tree small. Overlays (palette,
 * context menu, modal scrim, tooltip) are captured as viewport screenshots instead, because what is
 * under review there is precisely how the overlay separates from the surface behind it.
 */
const THEME_SURFACES: ThemeSurface[] = [
    {
        key: 'settings-toggle-on',
        record: async (page, save) => {
            await activateTab(settingsTab(page))
            const row = page.locator(switchRowSelector(SWITCH_STATE.ON)).first()
            await row.scrollIntoViewIfNeeded()
            await save(row)
        },
    },
    {
        key: 'settings-toggle-off',
        record: async (page, save) => {
            await activateTab(settingsTab(page))
            const row = page.locator(switchRowSelector(SWITCH_STATE.OFF)).first()
            await row.scrollIntoViewIfNeeded()
            await save(row)
        },
    },
    {
        key: 'tooltip',
        record: async (page, save) => {
            await sidebarViewTab(page, SIDEBAR_VIEW_LABEL.FILES).hover()
            const tooltip = page.locator(TOOLTIP_CONTENT_SELECTOR).first()
            await expect(tooltip).toBeVisible({ timeout: OVERLAY_SETTLE_TIMEOUT_MS })
            await save()
            await page.mouse.move(POINTER_PARK_POINT.x, POINTER_PARK_POINT.y)
            await expect(tooltip).toBeHidden()
        },
    },
    {
        key: 'sidebar-selected-hover-row',
        record: async (page, save) => {
            await showSidebarView(page, SIDEBAR_VIEW_LABEL.FILES)
            const selectedRow = explorerTreeRow(page, EDITOR_FILE_NAME)
            await selectedRow.click()
            await expect(selectedRow).toHaveAttribute(EXPLORER_SELECTED_ATTRIBUTE, 'true')
            await explorerTreeRow(page, HOVERED_TREE_FILE_NAME).hover()
            await save(sidebarPanel(page))
        },
    },
    {
        key: 'tab-bar-active-inactive',
        record: async (page, save) => {
            await activateTab(editorTab(page))
            await save(editorTabBar(page))
        },
    },
    {
        key: 'editor-current-line',
        record: async (page, save) => {
            const editor = await focusEditor(page)
            await page.keyboard.press(DOCUMENT_START_CHORD)
            await expect(editor.locator(MONACO_CURRENT_LINE_SELECTOR).first()).toBeVisible()
            await save(editor)
        },
    },
    {
        key: 'editor-drag-selection',
        record: async (page, save) => {
            const editor = await focusEditor(page)
            await page.keyboard.press(DOCUMENT_START_CHORD)
            await dragWithin(page, editor, EDITOR_DRAG_START_OFFSET, EDITOR_DRAG_END_OFFSET)
            await expect(editor.locator(MONACO_SELECTION_SELECTOR).first()).toBeVisible()
            await save(editor)
        },
    },
    {
        /**
         * The find query is seeded from a one-character selection rather than typed: Monaco's
         * `seedSearchStringFromSelection` is left at its default (`'always'`), and typing into
         * WKWebView is the documented corruption path this harness avoids everywhere else
         * (`lib/monaco-clipboard.ts`). One character also guarantees several matches in the fixture
         * module, so both `findMatch` and `currentFindMatch` are on screen at once.
         */
        key: 'editor-find-match',
        record: async (page, save) => {
            const editor = await focusEditor(page)
            await page.keyboard.press(DOCUMENT_START_CHORD)
            await page.keyboard.press(EXTEND_SELECTION_KEY)
            await page.keyboard.press(EDITOR_FIND_CHORD)
            const findWidget = editor.locator(MONACO_FIND_WIDGET_SELECTOR).first()
            await expect(findWidget).toBeVisible({ timeout: OVERLAY_SETTLE_TIMEOUT_MS })
            await expect(editor.locator(MONACO_FIND_MATCH_SELECTOR).first()).toBeVisible()
            await save(editor)
            await page.keyboard.press(ESCAPE_KEY)
            await expect(findWidget).toBeHidden()
        },
    },
    {
        key: 'command-palette',
        record: async (page, save) => {
            await page.keyboard.press(KEY_CHORD.COMMAND_PALETTE)
            const dialog = page.getByRole('dialog')
            await dialog.waitFor({ state: 'visible' })
            await dialog.locator('input').first().fill(`${PALETTE_MODE_PREFIX.COMMAND}${PALETTE_SAMPLE_QUERY}`)
            await expect(page.getByRole('option').first()).toBeVisible({ timeout: OVERLAY_SETTLE_TIMEOUT_MS })
            await save()
            await page.keyboard.press(ESCAPE_KEY)
            await expect(dialog).toBeHidden()
        },
    },
    {
        key: 'context-menu',
        record: async (page, save) => {
            const menu = await openTabBarContextMenu(page)
            await save()
            await page.keyboard.press(ESCAPE_KEY)
            await expect(menu).toBeHidden()
        },
    },
    {
        /**
         * Matched by title substring rather than the exact `"{category}: {title}"` label spec 09
         * uses: this command carries a keymap id, so its palette row can also render a
         * `CommandShortcut` (`command-palette-commands-group.tsx`) whose text would break an
         * anchored match. No other command's title contains this phrase.
         */
        key: 'dialog-overlay',
        record: async (page, save) => {
            await runPaletteCommand(page, KEYBINDINGS_COMMAND_TITLE)
            const overlay = page.locator(DIALOG_OVERLAY_SELECTOR).first()
            await expect(overlay).toBeVisible({ timeout: OVERLAY_SETTLE_TIMEOUT_MS })
            await save()
            await page.keyboard.press(ESCAPE_KEY)
            await expect(overlay).toBeHidden()
        },
    },
    {
        key: 'git-panel',
        record: async (page, save) => {
            await showSidebarView(page, SIDEBAR_VIEW_LABEL.GIT)
            const changesHeader = page.locator(GIT_SECTION_HEADER_SELECTOR).filter({ hasText: CHANGES_SECTION_TITLE_PATTERN }).first()
            await expect(changesHeader).toBeVisible({ timeout: GIT_STATUS_SETTLE_TIMEOUT_MS })
            await save(sidebarPanel(page))
            await showSidebarView(page, SIDEBAR_VIEW_LABEL.FILES)
        },
    },
    {
        key: 'terminal-selection',
        record: async (page, save) => {
            await activateTab(terminalTab(page))
            const terminal = page.locator(XTERM_SELECTOR).first()
            await expect(terminal).toBeVisible()
            await terminal.click()
            await dragWithin(page, terminal, TERMINAL_DRAG_START_OFFSET, TERMINAL_DRAG_END_OFFSET)
            await save(terminal)
        },
    },
]

const restoreTheme = async (page: Page, themes: BundledThemeSummary[], themeId: string) => {
    const original = themes.find((theme) => theme.id === themeId)
    if (!original) return
    await applyTheme(page, original).catch((error: unknown) => void error)
}

test.describe.configure({ timeout: SPEC_TIMEOUT_MS })

/**
 * Coverage for `docs/acknowledge/2026-09-15-d61-theme-state-distinctness-contract.md` §1.E: an
 * artifact generator, not a pass/fail oracle for colors. It walks every bundled theme
 * (`theme_list`, the same catalog §1.A/§1.D lint) and files one screenshot per UI surface under
 * `e2e/artifacts/theme-matrix/<theme>/<surface>.png`, so the follow-up vision review
 * (`docs/quality-assurance/2026-09-15-theme-matrix-review.md`) can judge "state invisible / border
 * gone / text buried" per surface and feed corrections back into the §1.A pair table and §1.B
 * contrast pairs.
 *
 * It is deliberately failure-tolerant in the middle and strict at the end: a surface that throws is
 * recorded and the walk continues (one missing PNG, not a lost row), a theme whose *switch* fails is
 * skipped entirely (screenshots filed under the wrong theme would be worse than none), and two time
 * budgets — per theme and for the whole matrix — bound a run that would otherwise scale with however
 * many themes the catalog has grown to. The collected list is asserted empty only after every
 * capture has been attempted, so a red run still leaves a complete-as-possible artifact tree.
 *
 * `bun run e2e` cannot be started by this harness (§0 — the user launches
 * `TAIDE_E2E_NO_HMR=1 bun run tauri dev` first); run this one alone with
 * `bun run e2e --grep theme-surface-matrix`, since at ~13 screenshots per theme it is far longer
 * than the rest of the suite combined.
 */
test('번들 테마 전수 × UI 표면별 스크린샷을 e2e/artifacts/theme-matrix 아래에 남긴다', async ({ page, fixtureProject }) => {
    await appendFile(path.join(fixtureProject.rootDir, DIRTY_FILE_NAME), GIT_DIRTY_MARKER_LINE, 'utf8')

    const settings = await invokeIpc<MatrixSettings>(page, 'settings_get')
    expect(settings.followSystemTheme ?? false, FOLLOW_SYSTEM_THEME_PRECONDITION_MESSAGE).toBe(false)
    const originalThemeId = settings.themeId ?? null

    await openFileViaQuickOpen(page, EDITOR_FILE_NAME)
    await page.keyboard.press(KEY_CHORD.NEW_TERMINAL)
    await expect(terminalTab(page)).toBeVisible()
    await runPaletteCommand(page, SETTINGS_COMMAND_LABEL, { exact: true })
    await expect(settingsTab(page)).toBeVisible()

    const allThemes = await invokeIpc<BundledThemeSummary[]>(page, 'theme_list')
    const bundledThemes = allThemes.filter((theme) => theme.builtin)
    expect(bundledThemes.length, BUNDLED_THEME_PRECONDITION_MESSAGE).toBeGreaterThan(0)

    const failures: MatrixFailure[] = []
    const matrixStartedAt = Date.now()

    try {
        for (const theme of bundledThemes) {
            if (Date.now() - matrixStartedAt > MATRIX_TIME_BUDGET_MS) {
                failures.push({ themeId: theme.id, surface: MATRIX_BUDGET_STEP, reason: MATRIX_BUDGET_REASON })
                continue
            }

            try {
                await applyTheme(page, theme)
            } catch (themeError) {
                failures.push({ themeId: theme.id, surface: THEME_SWITCH_STEP, reason: describeFailure(themeError) })
                continue
            }

            const themeStartedAt = Date.now()
            for (const surface of THEME_SURFACES) {
                if (Date.now() - themeStartedAt > THEME_TIME_BUDGET_MS) {
                    failures.push({ themeId: theme.id, surface: surface.key, reason: THEME_BUDGET_REASON })
                    continue
                }
                try {
                    await surface.record(page, (target) => captureSurface(page, theme.id, surface.key, target))
                } catch (surfaceError) {
                    failures.push({ themeId: theme.id, surface: surface.key, reason: describeFailure(surfaceError) })
                }
            }
        }
    } finally {
        if (originalThemeId) await restoreTheme(page, allThemes, originalThemeId)
    }

    expect(failures, MATRIX_FAILURE_MESSAGE).toEqual([])
})
