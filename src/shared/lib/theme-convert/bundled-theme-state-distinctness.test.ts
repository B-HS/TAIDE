import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, test } from 'bun:test'
import type { Theme_Serialize } from '@shared/api/bindings'
import { repairStateDistinctness, validateStateDistinctness } from '@shared/lib/theme-convert/state-distinctness'
import { STATE_DISTINCTNESS_PAIRS } from '@shared/lib/theme-convert/state-distinctness-pairs'
import { COLOR_NAMESPACES } from '@shared/lib/theme-convert/ui-token-vocabulary'

const BUNDLED_THEMES_DIR = join(import.meta.dir, '../../../../src-tauri/resources/themes')

const readBundledThemes = () =>
    readdirSync(BUNDLED_THEMES_DIR)
        .filter((name) => name.endsWith('.json'))
        .map((name) => JSON.parse(readFileSync(join(BUNDLED_THEMES_DIR, name), 'utf-8')) as Theme_Serialize)

/**
 * The catalog gate for `state-distinctness-pairs.ts`. Unlike the contrast gate next door
 * (`bundled-theme-contrast.test.ts`), this one carries no exemption registry: every axis it checks is
 * repairable by construction — `repairStateDistinctness` walks the container toward the theme's own
 * body foreground until the pair clears, and a theme whose foreground cannot be told apart from its
 * own background is already rejected by `validateOutputColors`. An exemption would therefore always
 * mean "the repair was not run", not "this theme legitimately collapses the state", so the fix is to
 * run `bun run themes:repair-state-distinctness` rather than to register the theme here.
 */
describe('번들 테마 상태색 구별성 게이트', () => {
    test('src-tauri/resources/themes/*.json 전량이 validateStateDistinctness 를 통과한다', () => {
        const themes = readBundledThemes()
        expect(themes.length).toBeGreaterThan(0)

        const violations = themes.flatMap((theme) => validateStateDistinctness(theme.colors).map((error) => `'${theme.id}': ${error}`))

        expect(violations).toEqual([])
    })

    test('번들 테마에 수리기를 다시 돌려도 바꿀 것이 없다(repair 스크립트 결과가 커밋된 상태)', () => {
        const repairs = readBundledThemes().flatMap((theme) =>
            repairStateDistinctness(theme.colors, theme.palette ?? {}).repairs.map((repair) => `'${theme.id}': ${repair}`),
        )

        expect(repairs).toEqual([])
    })

    /**
     * `theme.terminal` is the copy xterm reads (`xterm-theme.ts`), mirrored from the `terminal.*`
     * entries of `theme.colors` that this lint measures. `scripts/repair-theme-state-distinctness.ts`
     * writes both, and this catches a future repair (or hand edit) that moves only one of them and
     * leaves the terminal rendering the old, collapsed color while the gate reports success.
     */
    test('번들 테마의 terminal 미러가 colors 의 terminal.* 와 일치한다', () => {
        const drifted = readBundledThemes().flatMap((theme) =>
            (['background', 'foreground', 'cursor', 'selection'] as const)
                .filter((key) => theme.terminal[key] !== theme.colors[`terminal.${key}`])
                .map(
                    (key) =>
                        `'${theme.id}': terminal.${key}(${theme.terminal[key]}) != colors['terminal.${key}'](${theme.colors[`terminal.${key}`]})`,
                ),
        )

        expect(drifted).toEqual([])
    })
})

describe('상태색 구별성 쌍 표', () => {
    test('라벨이 중복되지 않는다(Rust 미러의 드리프트 검사 기준)', () => {
        const labels = STATE_DISTINCTNESS_PAIRS.map((pair) => pair.label)

        expect(new Set(labels).size).toBe(labels.length)
    })

    test('모든 쌍이 실제 토큰 어휘의 키만 참조한다', () => {
        const vocabulary = new Set(COLOR_NAMESPACES.flatMap(({ id, tokens }) => tokens.map((token) => `${id}.${token}`)))

        const unknown = STATE_DISTINCTNESS_PAIRS.flatMap((pair) =>
            [pair.stateKey, pair.containerKey, pair.surfaceKey, pair.alternativeStateKey]
                .filter((key): key is string => key !== undefined)
                .filter((key) => !vocabulary.has(key))
                .map((key) => `${pair.label}: ${key}`),
        )

        expect(unknown).toEqual([])
    })
})
