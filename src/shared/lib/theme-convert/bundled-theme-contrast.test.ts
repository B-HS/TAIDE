import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, test } from 'bun:test'
import type { Theme_Serialize } from '@shared/api/bindings'
import { validateOutputColors, validateSelectionRowContrast } from '@shared/lib/theme-convert/contrast'

const BUNDLED_THEMES_DIR = join(import.meta.dir, '../../../../src-tauri/resources/themes')

/**
 * Bundled themes whose `panel.matchHighlight` cannot clear `MIN_CONTRAST_RATIO`
 * (contrast.ts) against `panel.background` without abandoning the theme's own
 * accent hue. In both cases the upstream source defines its accent — VS Code's
 * `list.highlightForeground` — as exactly one shade, with no darker same-hue
 * variant available to substitute (unlike `github-dark`/`github-light`, whose
 * upstream ships a full lightness scale for the accent color). Falling back to
 * `repairContrastPairs`'s generic `editor.foreground` candidate is deliberately
 * not applied to bundled data, since it would replace the accent with a neutral
 * gray and erase the theme's identity. See
 * docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md §3-A for the
 * per-theme upstream palette investigation.
 */
const MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS: Record<string, string> = {
    'everforest-light':
        "upstream foreground palette (sainnhe/everforest-vscode src/palette/light/foreground.ts) has one shade per named accent — 'green' (#8da101, the source of list.highlightForeground) has no darker variant, only the lighter 'dimGreen' (#a4bb4a)",
    'rose-pine-dawn':
        "upstream Rose Pine Dawn palette defines exactly one shade per named color — 'rose' (#d7827e, the source of list.highlightForeground) has no darker variant; the nearest hue, 'love' (#b4637a), is a distinct accent already used for errors, not a shade of rose",
}

const readBundledThemeColors = (fileName: string) => {
    const parsed = JSON.parse(readFileSync(join(BUNDLED_THEMES_DIR, fileName), 'utf-8')) as Theme_Serialize
    return parsed.colors
}

describe('번들 테마 대비 게이트', () => {
    test('src-tauri/resources/themes/*.json 전량이 validateOutputColors 를 통과한다(예외 명시분 제외)', () => {
        const files = readdirSync(BUNDLED_THEMES_DIR).filter((name) => name.endsWith('.json'))
        expect(files.length).toBeGreaterThan(0)

        const violations = files
            .map((file) => ({ id: file.replace(/\.json$/, ''), colors: readBundledThemeColors(file) }))
            .filter(({ id }) => !(id in MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS))
            .flatMap(({ id, colors }) => validateOutputColors(colors).map((error) => `'${id}': ${error}`))

        expect(violations).toEqual([])
    })

    test('예외 등재분(everforest-light·rose-pine-dawn)은 실제로 matchHighlight 대비 부족 사유로만 위반한다', () => {
        for (const id of Object.keys(MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS)) {
            const errors = validateOutputColors(readBundledThemeColors(`${id}.json`))
            expect(errors.length).toBe(1)
            expect(errors[0]).toContain('matchHighlight')
        }
    })
})

/**
 * Selection-row axes (`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §0/§1-b):
 * `panel.matchHighlight`/`list.foreground` against `list.activeBackground`, the row surface a
 * selected palette/list entry actually paints (`docs/theme-system.md` §8.2, d-36 §4's render-path
 * confirmation). Both bundled themes below already carry a `panel.matchHighlight` exemption above for
 * the pre-existing `panel.background` axis — the reasons here are the same underlying upstream
 * palette shortfall, re-verified against the different background.
 *
 * `nord` is deliberately not listed here (d-40 review findings d40-listfg-multisurface-regression/
 * d40-l2-01/D40-L3-01/D40-L3-02): the exemption's premise held `list.activeBackground` fixed at the
 * theme's bright 'frost' accent (`#88c0d0`), but that same accent is also `panel.matchHighlight`,
 * `app.accent`, and `explorer.itemSelected` — changing it was never actually off the table. Moving
 * `list.activeBackground` to nord3 (`#4c566a`, the theme's own `list.inactiveSelectionBackground`)
 * clears both selection-row axes at once (`list.foreground` 5.46:1, `panel.matchHighlight` 3.69:1)
 * without an exemption, and stays perceptually distinct from `list.hoverBackground`/`list.background`
 * (ΔE 8.8/15.5) — see `src-tauri/resources/themes/nord.json`.
 */
const SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS: Record<string, string> = {
    'everforest-light':
        "same root cause as this theme's pre-existing panel.background exemption above — upstream sainnhe/everforest-vscode's foreground palette has exactly one shade for 'green' (#8da101, the source of list.highlightForeground/panel.matchHighlight), no darker variant to substitute. list.activeBackground (#e6e2cc, a ~50%-alpha overlay the gate reads at its raw opaque RGB) is darker than panel.background (raw #e6e2cc, L 0.756 vs #fdf6e3, L 0.923), so the same single-shade green loses contrast rather than gaining it.",
    'rose-pine-dawn':
        "upstream rose-pine/vscode defines list.activeSelectionBackground as a near-transparent overlay (#6e6a8614, ~8% alpha) — TAIDE's contrast gate measures an alpha-carrying background at its raw RGB (#6e6a86), which reads far darker than the overlay's actual on-screen appearance over the light base. Combined with this theme's pre-existing panel.background shortfall (rose, #d7827e, has no darker upstream variant — see the exemption above), no candidate clears 3:1 against the gate's raw-RGB reading of list.activeBackground either.",
}

/**
 * `list.foreground` against `list.activeBackground` — same render-path as
 * {@link SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS} above, for the row's non-matched text instead
 * of the search/palette match glyphs.
 */
const SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS: Record<string, string> = {
    'rose-pine-dawn':
        "same gate-vs-render mismatch as this theme's selectionMatchHighlight exemption above: even upstream's own dedicated list.activeSelectionForeground (#575279, already reused elsewhere in this file as app.foreground) falls short of 3:1 against the gate's raw-RGB reading of the near-transparent list.activeBackground (#6e6a86) — the shortfall is in how a translucent background is measured, not in the foreground choice.",
}

describe('번들 테마 대비 게이트 — 선택 행 축(list.activeBackground)', () => {
    test('src-tauri/resources/themes/*.json 전량이 validateSelectionRowContrast 를 통과한다(예외 명시분 제외)', () => {
        const files = readdirSync(BUNDLED_THEMES_DIR).filter((name) => name.endsWith('.json'))

        const violations = files
            .map((file) => ({ id: file.replace(/\.json$/, ''), colors: readBundledThemeColors(file) }))
            .flatMap(({ id, colors }) =>
                validateSelectionRowContrast(colors)
                    .filter((error) => {
                        if (error.includes('selectionMatchHighlight')) return !(id in SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS)
                        if (error.includes('selectionForeground')) return !(id in SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS)
                        return true
                    })
                    .map((error) => `'${id}': ${error}`),
            )

        expect(violations).toEqual([])
    })

    test('예외 등재분은 실제로 선언된 축에서만, 그 사유로만 위반한다', () => {
        /**
         * Derived from the two exemption `Record`s above rather than a hand-maintained literal list
         * (d-40 review finding D40-L3-06) — a theme added to either `Record` is picked up here
         * automatically, so an id/label pair can never drift out of sync with the registries
         * themselves.
         */
        const expectedLabelsById = new Map<string, string[]>()
        for (const id of Object.keys(SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS)) {
            expectedLabelsById.set(id, [...(expectedLabelsById.get(id) ?? []), 'selectionMatchHighlight'])
        }
        for (const id of Object.keys(SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS)) {
            expectedLabelsById.set(id, [...(expectedLabelsById.get(id) ?? []), 'selectionForeground'])
        }

        expect(expectedLabelsById.size).toBeGreaterThan(0)

        for (const [id, expectedLabels] of expectedLabelsById) {
            const errors = validateSelectionRowContrast(readBundledThemeColors(`${id}.json`))
            expect(errors.length).toBe(expectedLabels.length)
            for (const label of expectedLabels) expect(errors.some((error) => error.includes(label))).toBe(true)
        }

        const reasonsById = {
            ...SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS,
            ...SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS,
        }
        for (const reason of Object.values(reasonsById)) expect(reason.length).toBeGreaterThan(0)
    })
})

/**
 * `list.foreground` identical-color lint (d-40 review findings d40-listfg-multisurface-regression/
 * d40-l2-01/D40-L3-01) — `list.foreground` paints over `list.background` (unselected rows) and
 * `list.hoverBackground` (`--accent-foreground` on `--accent`, i.e. hovered rows, dropdown/context
 * menu items, ghost-button hover/focus text) just as much as it does over `list.activeBackground`.
 * Mirrors the three existing identical-color lints in this codebase (`app.foreground`==`app.background`,
 * `panel.matchHighlight`==`app.foreground`, `list.activeBackground`==`panel.background`/
 * `list.hoverBackground`, all in `src-tauri/src/domain/theme/service.rs`) — this axis had none until
 * nord shipped `list.foreground` hex-identical to `list.background` (contrast 1.00) undetected.
 */
describe('번들 테마 대비 게이트 — list.foreground 동일색', () => {
    test('src-tauri/resources/themes/*.json 전량이 list.foreground 를 list.background/list.hoverBackground 와 다른 색으로 갖는다', () => {
        const files = readdirSync(BUNDLED_THEMES_DIR).filter((name) => name.endsWith('.json'))
        expect(files.length).toBeGreaterThan(0)

        const violations = files
            .map((file) => ({ id: file.replace(/\.json$/, ''), colors: readBundledThemeColors(file) }))
            .flatMap(({ id, colors }) => {
                const errors: string[] = []
                if (colors['list.foreground'] === colors['list.background']) {
                    errors.push(
                        `'${id}': list.foreground(${colors['list.foreground']}) == list.background(${colors['list.background']}) — the unselected row's text would be invisible`,
                    )
                }
                if (colors['list.foreground'] === colors['list.hoverBackground']) {
                    errors.push(
                        `'${id}': list.foreground(${colors['list.foreground']}) == list.hoverBackground(${colors['list.hoverBackground']}) — hovering a row (or a --accent menu/button) would make its text disappear`,
                    )
                }
                return errors
            })

        expect(violations).toEqual([])
    })
})
