import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, test } from 'bun:test'
import type { Theme_Serialize } from '@shared/api/bindings'
import { validateOutputColors } from '@shared/lib/theme-convert/contrast'

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
