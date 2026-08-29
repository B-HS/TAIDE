import { describe, expect, test } from 'bun:test'
import { isSearchResultTruncated, resolveSearchResultsView } from '@entities/search/search-run-state'
import { SEARCH_MATCH_LIMIT } from '@shared/constants/search'

describe('resolveSearchResultsView', () => {
    test('아직 실행하지 않았으면 0건이 아니라 안내를 보여준다', () => {
        expect(resolveSearchResultsView({ status: 'idle', hasResults: false })).toBe('hint')
    })

    test('실패한 검색을 0건과 구분한다', () => {
        expect(resolveSearchResultsView({ status: 'failed', hasResults: false })).toBe('failed')
    })

    test('완료했는데 결과가 없으면 0건이다', () => {
        expect(resolveSearchResultsView({ status: 'completed', hasResults: false })).toBe('empty')
    })

    test('실행 중이고 아직 결과가 없으면 검색 중이다', () => {
        expect(resolveSearchResultsView({ status: 'running', hasResults: false })).toBe('searching')
    })

    test('결과가 도착했으면 실행 중이어도 결과를 보여준다', () => {
        expect(resolveSearchResultsView({ status: 'running', hasResults: true })).toBe('results')
        expect(resolveSearchResultsView({ status: 'completed', hasResults: true })).toBe('results')
    })
})

describe('isSearchResultTruncated', () => {
    test('상한 미만은 잘리지 않은 것이다', () => {
        expect(isSearchResultTruncated(0)).toBe(false)
        expect(isSearchResultTruncated(SEARCH_MATCH_LIMIT - 1)).toBe(false)
    })

    test('상한에 도달하면 잘린 것으로 본다', () => {
        expect(isSearchResultTruncated(SEARCH_MATCH_LIMIT)).toBe(true)
    })
})
