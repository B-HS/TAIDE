import { describe, expect, test } from 'bun:test'
import type { SearchQuery } from '@shared/api/bindings'
import { isSameSearchQuery, normalizeSearchQuery } from '@entities/search/search-query'

const panelQuery = (overrides: Partial<SearchQuery> = {}): SearchQuery => ({
    text: 'needle',
    caseSensitive: false,
    wholeWord: false,
    regex: false,
    includeGlob: null,
    excludeGlob: null,
    respectGitignore: true,
    ...overrides,
})

describe('normalizeSearchQuery', () => {
    test('생략된 옵션을 기본값으로 채운다', () => {
        expect(normalizeSearchQuery({ text: 'needle' })).toEqual({
            text: 'needle',
            caseSensitive: false,
            wholeWord: false,
            regex: false,
            includeGlob: null,
            excludeGlob: null,
            contextLines: 0,
            respectGitignore: true,
        })
    })

    test('명시된 옵션은 그대로 둔다', () => {
        expect(normalizeSearchQuery({ text: 'needle', caseSensitive: true, contextLines: 3 })).toMatchObject({
            caseSensitive: true,
            contextLines: 3,
        })
    })
})

describe('isSameSearchQuery', () => {
    test('옵션이 생략된 쪽과 기본값으로 채운 쪽을 같다고 본다', () => {
        expect(isSameSearchQuery({ text: 'needle' }, panelQuery())).toBe(true)
    })

    test('검색어가 다르면 다르다', () => {
        expect(isSameSearchQuery(panelQuery(), panelQuery({ text: 'other' }))).toBe(false)
    })

    test('토글이 하나라도 다르면 다르다', () => {
        expect(isSameSearchQuery(panelQuery(), panelQuery({ caseSensitive: true }))).toBe(false)
        expect(isSameSearchQuery(panelQuery(), panelQuery({ wholeWord: true }))).toBe(false)
        expect(isSameSearchQuery(panelQuery(), panelQuery({ regex: true }))).toBe(false)
        expect(isSameSearchQuery(panelQuery(), panelQuery({ respectGitignore: false }))).toBe(false)
    })

    test('포함·제외 글롭이 다르면 다르다', () => {
        expect(isSameSearchQuery(panelQuery(), panelQuery({ includeGlob: 'src/**' }))).toBe(false)
        expect(isSameSearchQuery(panelQuery(), panelQuery({ excludeGlob: '*.test.ts' }))).toBe(false)
    })

    test('컨텍스트 줄 수는 매치 집합을 바꾸지 않으므로 비교하지 않는다', () => {
        expect(isSameSearchQuery(panelQuery({ contextLines: 0 }), panelQuery({ contextLines: 3 }))).toBe(true)
    })
})
