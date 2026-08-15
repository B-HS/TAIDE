import { describe, expect, test } from 'bun:test'
import { registerTerminalWriteHandler, requestTerminalWrite } from '@shared/lib/terminal-write-bridge'

describe('terminalWriteBridge / 핸들러가 등록된 상태', () => {
    test('요청하면 즉시 핸들러가 호출된다', () => {
        const received: string[] = []
        const unregister = registerTerminalWriteHandler('tab-1', (data) => received.push(data))

        requestTerminalWrite('tab-1', 'npm run build\n')
        unregister()

        expect(received).toEqual(['npm run build\n'])
    })

    test('등록 해제 후에는 호출되지 않고 다시 대기열에 쌓인다', () => {
        const received: string[] = []
        const unregister = registerTerminalWriteHandler('tab-1', (data) => received.push(data))
        unregister()

        requestTerminalWrite('tab-1', 'echo hi\n')

        expect(received).toEqual([])
    })

    test('서로 다른 tabId 의 핸들러는 독립적으로 호출된다', () => {
        const receivedA: string[] = []
        const receivedB: string[] = []
        const unregisterA = registerTerminalWriteHandler('tab-a', (data) => receivedA.push(data))
        const unregisterB = registerTerminalWriteHandler('tab-b', (data) => receivedB.push(data))

        requestTerminalWrite('tab-a', 'a\n')
        requestTerminalWrite('tab-b', 'b\n')
        unregisterA()
        unregisterB()

        expect(receivedA).toEqual(['a\n'])
        expect(receivedB).toEqual(['b\n'])
    })
})

describe('terminalWriteBridge / 핸들러 등록 전 요청(대기열)', () => {
    test('핸들러 등록 전 요청은 대기열에 쌓였다가 등록 시 순서대로 flush 된다', () => {
        requestTerminalWrite('tab-2', 'first\n')
        requestTerminalWrite('tab-2', 'second\n')

        const received: string[] = []
        const unregister = registerTerminalWriteHandler('tab-2', (data) => received.push(data))
        unregister()

        expect(received).toEqual(['first\n', 'second\n'])
    })

    test('flush 된 대기열은 다시 재전달되지 않는다', () => {
        requestTerminalWrite('tab-3', 'only\n')

        let firstCalls = 0
        const unregisterFirst = registerTerminalWriteHandler('tab-3', () => {
            firstCalls += 1
        })
        unregisterFirst()

        let secondCalls = 0
        const unregisterSecond = registerTerminalWriteHandler('tab-3', () => {
            secondCalls += 1
        })
        unregisterSecond()

        expect(firstCalls).toBe(1)
        expect(secondCalls).toBe(0)
    })
})
