import { describe, expect, test } from 'bun:test'
import { requestRevealInExplorer, subscribeRevealInExplorer } from '@shared/lib/explorer-reveal-bridge'

describe('explorerRevealBridge', () => {
    test('요청한 path 가 그대로 리스너에 전달된다', () => {
        let received: string | undefined
        const unsubscribe = subscribeRevealInExplorer((path) => {
            received = path
        })

        requestRevealInExplorer('/repo/src/main.ts')
        unsubscribe()

        expect(received).toBe('/repo/src/main.ts')
    })

    test('여러 리스너가 모두 호출된다', () => {
        let first: string | undefined
        let second: string | undefined
        const unsubscribeFirst = subscribeRevealInExplorer((path) => {
            first = path
        })
        const unsubscribeSecond = subscribeRevealInExplorer((path) => {
            second = path
        })

        requestRevealInExplorer('/repo/a.ts')
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe('/repo/a.ts')
        expect(second).toBe('/repo/a.ts')
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeRevealInExplorer(() => {
            calls += 1
        })
        unsubscribe()

        requestRevealInExplorer('/repo/a.ts')

        expect(calls).toBe(0)
    })
})
