import { describe, expect, test } from 'bun:test'
import { kindContains, kindMatchesAny } from '@shared/lib/lsp/kind'

describe('kindContains', () => {
    test('동일한 kind 는 서로 포함한다', () => {
        expect(kindContains('source.organizeImports', 'source.organizeImports')).toBe(true)
    })

    test('상위 kind 는 하위 kind 를 포함한다', () => {
        expect(kindContains('source.organizeImports', 'source.organizeImports.ts')).toBe(true)
        expect(kindContains('refactor', 'refactor.extract.function')).toBe(true)
    })

    test('빈 문자열 kind 는 모든 kind 를 포함한다', () => {
        expect(kindContains('', 'quickfix')).toBe(true)
        expect(kindContains('', '')).toBe(true)
    })

    test('접두사만 같고 계층 구분자가 없으면 포함하지 않는다', () => {
        expect(kindContains('source.organizeImports', 'source.organizeImportsExtra')).toBe(false)
    })

    test('무관한 kind 는 포함하지 않는다', () => {
        expect(kindContains('quickfix', 'refactor')).toBe(false)
    })

    test('하위 kind 는 상위 kind 를 포함하지 않는다 (비대칭)', () => {
        expect(kindContains('source.organizeImports.ts', 'source.organizeImports')).toBe(false)
    })
})

describe('kindMatchesAny', () => {
    test('여러 parent 중 하나라도 포함하면 true 를 반환한다', () => {
        expect(kindMatchesAny(['quickfix', 'source.fixAll'], 'source.fixAll.eslint')).toBe(true)
    })

    test('어느 parent 도 포함하지 않으면 false 를 반환한다', () => {
        expect(kindMatchesAny(['quickfix', 'refactor'], 'source.organizeImports')).toBe(false)
    })

    test('parent 목록이 비어 있으면 false 를 반환한다', () => {
        expect(kindMatchesAny([], 'quickfix')).toBe(false)
    })
})
