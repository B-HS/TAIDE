import { describe, expect, test } from 'bun:test'
import { flushAllLspSessions, registerLspSessionAllFlush } from '@widgets/editor-pane/lsp-session-flush-registry'

describe('flushAllLspSessions', () => {
    test('등록된 핸들러를 호출한다', () => {
        let calls = 0
        registerLspSessionAllFlush(() => {
            calls += 1
        })

        flushAllLspSessions()

        expect(calls).toBe(1)
    })

    test('나중에 등록한 핸들러가 이전 핸들러를 대체한다', () => {
        let firstCalls = 0
        let secondCalls = 0
        registerLspSessionAllFlush(() => {
            firstCalls += 1
        })
        registerLspSessionAllFlush(() => {
            secondCalls += 1
        })

        flushAllLspSessions()

        expect(firstCalls).toBe(0)
        expect(secondCalls).toBe(1)
    })
})
