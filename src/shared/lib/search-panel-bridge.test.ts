import { describe, expect, test } from 'bun:test'
import { requestOpenSearchPanel, subscribeOpenSearchPanel } from '@shared/lib/search-panel-bridge'

describe('searchPanelBridge', () => {
    test('구독한 리스너는 요청 시 호출된다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenSearchPanel(() => {
            calls += 1
        })

        requestOpenSearchPanel()
        requestOpenSearchPanel()
        unsubscribe()

        expect(calls).toBe(2)
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenSearchPanel(() => {
            calls += 1
        })
        unsubscribe()

        requestOpenSearchPanel()

        expect(calls).toBe(0)
    })

    test('여러 리스너가 모두 호출된다', () => {
        let first = 0
        let second = 0
        const unsubscribeFirst = subscribeOpenSearchPanel(() => {
            first += 1
        })
        const unsubscribeSecond = subscribeOpenSearchPanel(() => {
            second += 1
        })

        requestOpenSearchPanel()
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe(1)
        expect(second).toBe(1)
    })

    test('부분 요청은 나머지 필드가 기본값으로 채워진다', () => {
        let received: unknown
        const unsubscribe = subscribeOpenSearchPanel((request) => {
            received = request
        })

        requestOpenSearchPanel({ includeGlob: 'src/**' })
        unsubscribe()

        expect(received).toEqual({ includeGlob: 'src/**', seedText: null, openReplace: false })
    })
})
