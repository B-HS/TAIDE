import { describe, expect, test } from 'bun:test'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import type { KeybindingRow } from '@shared/lib/keymap/keybinding-catalog'
import { buildKeybindingConflictIndex, buildKeybindingRows, findConflictingRowInIndex } from '@shared/lib/keymap/keybinding-catalog'
import type { KeymapModifier, KeymapOverrideEntry } from '@shared/lib/keymap/keymap'
import { CountingMap } from '@shared/testing/counting-map'

/**
 * Budget tests: they count operations, never milliseconds. Wall-clock assertions belong in
 * `scripts/bench-frontend.ts`, which is run by hand — a timing threshold in `bun test` fails on a
 * loaded CI runner for reasons that have nothing to do with the code
 * (`docs/quality-assurance/2026-09-04-perf-baseline.md` §5).
 */
const BINDING_KEYS = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h']
const BINDING_MODS: KeymapModifier[][] = [['mod'], ['mod', 'shift'], ['alt']]

const buildBoundRows = (size: number) => {
    const commands = Array.from({ length: size }, (_, index) => ({ id: `budget.command-${index}`, titleKey: 'budget.command', run: () => {} }))
    const overrides: KeymapOverrideEntry[] = commands.map((command, index) => ({
        actionId: command.id,
        key: BINDING_KEYS[index % BINDING_KEYS.length],
        mods: BINDING_MODS[index % BINDING_MODS.length],
    }))
    return buildKeybindingRows(commands, overrides).filter((row) => row.commandId !== null)
}

/** Re-wraps a real index in a counting map so a lookup's map reads can be counted without touching production code. */
const toCountingIndex = (rows: KeybindingRow[]) => {
    const source = buildKeybindingConflictIndex(rows, true)
    const rowsByBinding = new CountingMap<string, KeybindingRow[]>()
    for (const [signature, bucket] of source.rowsByBinding) rowsByBinding.set(signature, bucket)
    rowsByBinding.resetCounts()
    return { index: { rowsByBinding, isMac: source.isMac }, rowsByBinding }
}

describe('fuzzyFilter 연산 예산', () => {
    test('후보마다 라벨을 정확히 한 번만 읽는다 — 파생 라벨 재계산 금지', () => {
        const candidates = Array.from({ length: 500 }, (_, index) => ({ path: `src/widgets/panel-${index}/view.tsx` }))
        let labelReads = 0

        fuzzyFilter('swp', candidates, (candidate) => {
            labelReads += 1
            return candidate.path
        })

        expect(labelReads).toBe(candidates.length)
    })

    test('질의가 비어 있어도 후보당 라벨 1회를 넘지 않는다', () => {
        const candidates = Array.from({ length: 50 }, (_, index) => ({ path: `file-${index}.ts` }))
        let labelReads = 0

        fuzzyFilter('', candidates, (candidate) => {
            labelReads += 1
            return candidate.path
        })

        expect(labelReads).toBeLessThanOrEqual(candidates.length)
    })
})

describe('키바인딩 충돌 조회 연산 예산', () => {
    test('조회 1건은 카탈로그 크기와 무관하게 맵 조회 1회만 한다', () => {
        const rows = buildBoundRows(240)
        const { index, rowsByBinding } = toCountingIndex(rows)

        findConflictingRowInIndex(index, rows[0])

        expect(rowsByBinding.getCount).toBe(1)
    })

    test('카탈로그가 4배가 되어도 전수 조회의 맵 조회 수는 행 수에 선형이다 — O(n²) 스캔 회귀 가드', () => {
        const smallRows = buildBoundRows(60)
        const largeRows = buildBoundRows(240)
        const small = toCountingIndex(smallRows)
        const large = toCountingIndex(largeRows)

        for (const row of smallRows) findConflictingRowInIndex(small.index, row)
        for (const row of largeRows) findConflictingRowInIndex(large.index, row)

        expect(small.rowsByBinding.getCount).toBe(smallRows.length)
        expect(large.rowsByBinding.getCount).toBe(largeRows.length)
    })

    test('충돌 버킷은 같은 시그니처를 가진 행만 담는다 — 조회가 훑는 상한', () => {
        const rows = buildBoundRows(240)
        const index = buildKeybindingConflictIndex(rows, true)
        const signatureCount = BINDING_KEYS.length * BINDING_MODS.length

        expect(index.rowsByBinding.size).toBeLessThanOrEqual(signatureCount)
        for (const bucket of index.rowsByBinding.values())
            expect(bucket.length).toBeLessThanOrEqual(Math.ceil(rows.length / index.rowsByBinding.size))
    })
})
