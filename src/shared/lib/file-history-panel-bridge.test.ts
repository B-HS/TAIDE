import { describe, expect, test } from 'bun:test'
import { requestOpenFileHistory, subscribeOpenFileHistory } from '@shared/lib/file-history-panel-bridge'

describe('fileHistoryPanelBridge', () => {
    test('요청한 path 가 그대로 리스너에 전달된다', () => {
        let received: string | undefined
        const unsubscribe = subscribeOpenFileHistory((path) => {
            received = path
        })

        requestOpenFileHistory('/repo/src/main.ts')
        unsubscribe()

        expect(received).toBe('/repo/src/main.ts')
    })

    test('여러 리스너가 모두 호출된다', () => {
        let first: string | undefined
        let second: string | undefined
        const unsubscribeFirst = subscribeOpenFileHistory((path) => {
            first = path
        })
        const unsubscribeSecond = subscribeOpenFileHistory((path) => {
            second = path
        })

        requestOpenFileHistory('/repo/a.ts')
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe('/repo/a.ts')
        expect(second).toBe('/repo/a.ts')
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenFileHistory(() => {
            calls += 1
        })
        unsubscribe()

        requestOpenFileHistory('/repo/a.ts')

        expect(calls).toBe(0)
    })
})
