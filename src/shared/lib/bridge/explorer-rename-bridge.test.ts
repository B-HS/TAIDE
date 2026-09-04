import { describe, expect, test } from 'bun:test'
import { requestRenameInExplorer, subscribeRenameInExplorer } from '@shared/lib/bridge/explorer-rename-bridge'

describe('explorerRenameBridge', () => {
    test('요청한 path 가 그대로 리스너에 전달된다', () => {
        let received: string | undefined
        const unsubscribe = subscribeRenameInExplorer((path) => {
            received = path
        })

        requestRenameInExplorer('/repo/src/main.ts')
        unsubscribe()

        expect(received).toBe('/repo/src/main.ts')
    })

    test('여러 리스너가 모두 호출된다 — 사이드바 펼치기와 탐색기 rename 이 같은 요청을 나눠 받는다', () => {
        let first: string | undefined
        let second: string | undefined
        const unsubscribeFirst = subscribeRenameInExplorer((path) => {
            first = path
        })
        const unsubscribeSecond = subscribeRenameInExplorer((path) => {
            second = path
        })

        requestRenameInExplorer('/repo/a.ts')
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe('/repo/a.ts')
        expect(second).toBe('/repo/a.ts')
    })

    test('구독자가 없을 때의 요청은 버려지고 나중 구독자에게 재생되지 않는다', () => {
        requestRenameInExplorer('/repo/dropped.ts')

        let calls = 0
        const unsubscribe = subscribeRenameInExplorer(() => {
            calls += 1
        })
        unsubscribe()

        expect(calls).toBe(0)
    })

    test('요청마다 리스너가 한 번씩 호출된다', () => {
        const received: string[] = []
        const unsubscribe = subscribeRenameInExplorer((path) => {
            received.push(path)
        })

        requestRenameInExplorer('/repo/a.ts')
        requestRenameInExplorer('/repo/b.ts')
        unsubscribe()

        expect(received).toEqual(['/repo/a.ts', '/repo/b.ts'])
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeRenameInExplorer(() => {
            calls += 1
        })
        unsubscribe()

        requestRenameInExplorer('/repo/a.ts')

        expect(calls).toBe(0)
    })
})
