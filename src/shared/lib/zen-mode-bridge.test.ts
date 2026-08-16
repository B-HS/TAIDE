import { describe, expect, test } from 'bun:test'
import { requestToggleZenMode, subscribeToggleZenMode } from '@shared/lib/zen-mode-bridge'

describe('zenModeBridge / toggleZenMode', () => {
    test('구독한 리스너는 요청 시 호출된다', () => {
        let calls = 0
        const unsubscribe = subscribeToggleZenMode(() => {
            calls += 1
        })

        requestToggleZenMode()
        requestToggleZenMode()
        unsubscribe()

        expect(calls).toBe(2)
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeToggleZenMode(() => {
            calls += 1
        })
        unsubscribe()

        requestToggleZenMode()

        expect(calls).toBe(0)
    })

    test('여러 리스너가 모두 호출된다', () => {
        let firstCalls = 0
        let secondCalls = 0
        const unsubscribeFirst = subscribeToggleZenMode(() => {
            firstCalls += 1
        })
        const unsubscribeSecond = subscribeToggleZenMode(() => {
            secondCalls += 1
        })

        requestToggleZenMode()
        unsubscribeFirst()
        unsubscribeSecond()

        expect(firstCalls).toBe(1)
        expect(secondCalls).toBe(1)
    })
})
