import { describe, expect, test } from 'bun:test'
import { registerTerminalWriteHandler, requestTerminalWrite } from '@shared/lib/terminal-write-bridge'

const SHORT_TTL_MS = 20
const SMALL_MAX_ENTRIES = 3

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

describe('terminalWriteBridge / 대기열 상한(TERMINAL_WRITE_QUEUE_MAX_ENTRIES)', () => {
    test('상한을 넘겨 요청하면 가장 오래된 항목부터 버려지고 최근 항목만 flush 된다', () => {
        const tabId = 'tab-cap'
        requestTerminalWrite(tabId, '1', undefined, SMALL_MAX_ENTRIES)
        requestTerminalWrite(tabId, '2', undefined, SMALL_MAX_ENTRIES)
        requestTerminalWrite(tabId, '3', undefined, SMALL_MAX_ENTRIES)
        requestTerminalWrite(tabId, '4', undefined, SMALL_MAX_ENTRIES)
        requestTerminalWrite(tabId, '5', undefined, SMALL_MAX_ENTRIES)

        const received: string[] = []
        const unregister = registerTerminalWriteHandler(tabId, (data) => received.push(data))
        unregister()

        expect(received).toEqual(['3', '4', '5'])
    })
})

describe('terminalWriteBridge / 대기열 TTL(TERMINAL_WRITE_QUEUE_TTL_MS)', () => {
    test('TTL 이 지난 대기열 항목은 폐기되어 뒤늦게 등록해도 전달되지 않는다', async () => {
        const tabId = 'tab-ttl-expired'
        requestTerminalWrite(tabId, 'stale\n', SHORT_TTL_MS)
        await new Promise((resolve) => setTimeout(resolve, SHORT_TTL_MS * 3))

        const received: string[] = []
        const unregister = registerTerminalWriteHandler(tabId, (data) => received.push(data), SHORT_TTL_MS)
        unregister()

        expect(received).toEqual([])
    })

    test('TTL 이 지나기 전에 등록하면 정상적으로 전달된다', () => {
        const tabId = 'tab-ttl-fresh'
        requestTerminalWrite(tabId, 'fresh\n', SHORT_TTL_MS)

        const received: string[] = []
        const unregister = registerTerminalWriteHandler(tabId, (data) => received.push(data), SHORT_TTL_MS)
        unregister()

        expect(received).toEqual(['fresh\n'])
    })

    test('먼저 쌓인 항목이 TTL 로 만료되어도 그 뒤에 쌓인 신선한 항목은 살아남는다', async () => {
        const tabId = 'tab-ttl-partial'
        requestTerminalWrite(tabId, 'old\n', SHORT_TTL_MS)
        await new Promise((resolve) => setTimeout(resolve, SHORT_TTL_MS * 3))
        requestTerminalWrite(tabId, 'new\n', SHORT_TTL_MS)

        const received: string[] = []
        const unregister = registerTerminalWriteHandler(tabId, (data) => received.push(data), SHORT_TTL_MS)
        unregister()

        expect(received).toEqual(['new\n'])
    })
})
