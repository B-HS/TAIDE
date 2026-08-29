import { describe, expect, test } from 'bun:test'
import {
    addRecentSearchTerm,
    createRecentSearchQueue,
    enqueueRecentSearch,
    SEARCH_HISTORY_LIMIT,
    settleRecentSearch,
} from '@entities/search/search-history'

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

describe('enqueueRecentSearch', () => {
    test('설정 저장이 끝나기 전에 다음 검색어를 넣어도 앞선 검색어가 유실되지 않는다', () => {
        const queue = createRecentSearchQueue()
        const settled: string[] = []

        expect(enqueueRecentSearch(queue, settled, 'foo')).toEqual(['foo'])
        expect(enqueueRecentSearch(queue, settled, 'bar')).toEqual(['bar', 'foo'])
    })

    test('저장이 모두 끝나면 서버가 돌려준 값을 다시 기준으로 삼는다', () => {
        const queue = createRecentSearchQueue()

        enqueueRecentSearch(queue, [], 'foo')
        settleRecentSearch(queue)

        expect(enqueueRecentSearch(queue, ['foo', 'external'], 'bar')).toEqual(['bar', 'foo', 'external'])
    })

    test('저장이 여러 건 남아 있으면 마지막 하나가 끝날 때까지 기준을 유지한다', () => {
        const queue = createRecentSearchQueue()

        enqueueRecentSearch(queue, [], 'foo')
        enqueueRecentSearch(queue, [], 'bar')
        settleRecentSearch(queue)

        expect(enqueueRecentSearch(queue, [], 'baz')).toEqual(['baz', 'bar', 'foo'])
    })

    test('이미 맨 앞인 검색어는 저장을 예약하지 않는다', () => {
        const queue = createRecentSearchQueue()

        expect(enqueueRecentSearch(queue, ['foo'], 'foo')).toBeNull()
        expect(queue.inFlight).toBe(0)
    })
})
