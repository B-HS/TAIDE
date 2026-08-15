import { describe, expect, test } from 'bun:test'
import { addRecentSearchTerm, SEARCH_HISTORY_LIMIT } from '@entities/search/search-history'

describe('addRecentSearchTerm', () => {
    test('새 검색어를 맨 앞에 추가한다', () => {
        expect(addRecentSearchTerm(['b', 'a'], 'c')).toEqual(['c', 'b', 'a'])
    })

    test('이미 있는 검색어는 중복 제거 후 맨 앞으로 이동한다', () => {
        expect(addRecentSearchTerm(['a', 'b', 'c'], 'b')).toEqual(['b', 'a', 'c'])
    })

    test('앞뒤 공백은 잘라내고 저장한다', () => {
        expect(addRecentSearchTerm([], '  hello  ')).toEqual(['hello'])
    })

    test('빈 문자열이나 공백만 있는 검색어는 무시한다', () => {
        expect(addRecentSearchTerm(['a'], '')).toEqual(['a'])
        expect(addRecentSearchTerm(['a'], '   ')).toEqual(['a'])
    })

    test('이미 맨 앞에 있는 검색어를 다시 추가하면 원본 배열 참조를 그대로 반환한다', () => {
        const history = ['a', 'b', 'c']
        expect(addRecentSearchTerm(history, 'a')).toBe(history)
        expect(addRecentSearchTerm(history, '  a  ')).toBe(history)
    })

    test(`상한 ${SEARCH_HISTORY_LIMIT}개를 넘으면 가장 오래된 항목을 버린다`, () => {
        const full = Array.from({ length: SEARCH_HISTORY_LIMIT }, (_, index) => `term-${index}`)
        const result = addRecentSearchTerm(full, 'new-term')

        expect(result.length).toBe(SEARCH_HISTORY_LIMIT)
        expect(result[0]).toBe('new-term')
        expect(result).not.toContain(`term-${SEARCH_HISTORY_LIMIT - 1}`)
    })
})
