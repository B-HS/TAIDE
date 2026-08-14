import { describe, expect, test } from 'bun:test'
import { flushAllMirrors, registerMirrorFlush, unregisterMirrorFlush } from '@entities/editor/mirror-flush-registry'

describe('flushAllMirrors', () => {
    test('등록된 모든 탭의 flush 콜백을 호출한다', async () => {
        const calledTabIds: string[] = []
        registerMirrorFlush('tab-a', () => {
            calledTabIds.push('tab-a')
        })
        registerMirrorFlush('tab-b', () => {
            calledTabIds.push('tab-b')
        })

        await flushAllMirrors()

        expect(calledTabIds.sort()).toEqual(['tab-a', 'tab-b'])

        unregisterMirrorFlush('tab-a')
        unregisterMirrorFlush('tab-b')
    })

    test('등록 해제된 탭은 더 이상 호출되지 않는다', async () => {
        let calls = 0
        registerMirrorFlush('tab-c', () => {
            calls += 1
        })
        unregisterMirrorFlush('tab-c')

        await flushAllMirrors()

        expect(calls).toBe(0)
    })

    test('한 flush 가 실패해도 나머지 flush 는 모두 실행된다', async () => {
        let succeededCalls = 0
        registerMirrorFlush('tab-failing', () => {
            throw new Error('flush 실패')
        })
        registerMirrorFlush('tab-ok', () => {
            succeededCalls += 1
        })

        await expect(flushAllMirrors()).resolves.toBeUndefined()
        expect(succeededCalls).toBe(1)

        unregisterMirrorFlush('tab-failing')
        unregisterMirrorFlush('tab-ok')
    })

    test('비동기 flush 콜백의 완료를 기다린다', async () => {
        let resolved = false
        registerMirrorFlush('tab-async', async () => {
            await new Promise((resolve) => setTimeout(resolve, 0))
            resolved = true
        })

        await flushAllMirrors()

        expect(resolved).toBe(true)
        unregisterMirrorFlush('tab-async')
    })
})
