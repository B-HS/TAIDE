import { describe, expect, test } from 'bun:test'
import type { JsonRpcNotification, JsonRpcRequest } from '@shared/lib/lsp/protocol'
import { LspCapabilityNotSupportedError, LspDocumentNotOpenError, createLspClient } from '@shared/lib/lsp/client'

const createHarness = () => {
    const sent: (JsonRpcRequest | JsonRpcNotification)[] = []
    const notifications: JsonRpcNotification[] = []
    const client = createLspClient({
        send: (message) => sent.push(message),
        onNotification: (notification) => notifications.push(notification),
    })
    return { sent, notifications, client }
}

const respond = (id: number, result: unknown) => ({ jsonrpc: '2.0', id, result })

describe('createLspClient — request/response id 매칭', () => {
    test('요청에 발급한 id 와 응답의 id 를 매칭해 resolve 한다', async () => {
        const { sent, client } = createHarness()

        const pending = client.request<{ ok: boolean }>('workspace/symbol', { query: 'foo' })
        expect(sent).toHaveLength(1)
        const request = sent[0] as JsonRpcRequest
        expect(request.method).toBe('workspace/symbol')

        client.handleMessage(respond(request.id as number, { ok: true }))

        await expect(pending).resolves.toEqual({ ok: true })
    })

    test('id 가 일치하지 않는 응답은 무시된다', async () => {
        const { sent, client } = createHarness()

        const pending = client.request('workspace/symbol', {})
        const request = sent[0] as JsonRpcRequest

        client.handleMessage(respond(9999, { ignored: true }))
        client.handleMessage(respond(request.id as number, { matched: true }))

        await expect(pending).resolves.toEqual({ matched: true })
    })
})

describe('createLspClient — 동시 다중 요청', () => {
    test('여러 요청이 응답 순서와 무관하게 각자의 id 로 정확히 라우팅된다', async () => {
        const { sent, client } = createHarness()

        const first = client.request<string>('a/method', {})
        const second = client.request<string>('b/method', {})
        const third = client.request<string>('c/method', {})

        expect(sent).toHaveLength(3)
        const [firstId, secondId, thirdId] = sent.map((message) => (message as JsonRpcRequest).id as number)
        expect(new Set([firstId, secondId, thirdId]).size).toBe(3)

        client.handleMessage(respond(thirdId, 'third'))
        client.handleMessage(respond(firstId, 'first'))
        client.handleMessage(respond(secondId, 'second'))

        await expect(first).resolves.toBe('first')
        await expect(second).resolves.toBe('second')
        await expect(third).resolves.toBe('third')
    })
})

describe('createLspClient — 에러 응답 처리', () => {
    test('error 필드를 가진 응답은 pending 요청을 reject 한다', async () => {
        const { sent, client } = createHarness()

        const pending = client.request('workspace/symbol', {})
        const request = sent[0] as JsonRpcRequest

        client.handleMessage({ jsonrpc: '2.0', id: request.id, error: { code: -32601, message: 'method not found' } })

        await expect(pending).rejects.toEqual({ code: -32601, message: 'method not found' })
    })
})

describe('createLspClient — 응답 없는 알림 처리', () => {
    test('알림 메시지는 pending 요청 맵에 영향을 주지 않고 onNotification 으로 전달된다', () => {
        const { notifications, client } = createHarness()

        client.handleMessage({ jsonrpc: '2.0', method: 'window/logMessage', params: { message: 'hello' } })

        expect(notifications).toHaveLength(1)
        expect(notifications[0]).toEqual({ jsonrpc: '2.0', method: 'window/logMessage', params: { message: 'hello' } })
    })

    test('publishDiagnostics 알림은 onDiagnostics 구독자에게도 전달되고 onNotification 에도 전달된다', () => {
        const { notifications, client } = createHarness()
        const received: unknown[] = []
        client.onDiagnostics((params) => received.push(params))

        client.handleMessage({
            jsonrpc: '2.0',
            method: 'textDocument/publishDiagnostics',
            params: { uri: 'file:///a.ts', diagnostics: [] },
        })

        expect(received).toEqual([{ uri: 'file:///a.ts', diagnostics: [] }])
        expect(notifications).toHaveLength(1)
    })

    test('dispose 된 구독자는 더 이상 알림을 받지 않는다', () => {
        const { client } = createHarness()
        const received: unknown[] = []
        const subscription = client.onDiagnostics((params) => received.push(params))
        subscription.dispose()

        client.handleMessage({ jsonrpc: '2.0', method: 'textDocument/publishDiagnostics', params: { uri: 'file:///a.ts', diagnostics: [] } })

        expect(received).toHaveLength(0)
    })
})

describe('createLspClient — capabilities 미지원 시 요청 안 보냄', () => {
    test('initialize 이전에는 capability 가 필요한 요청이 즉시 reject 되고 전송되지 않는다', async () => {
        const { sent, client } = createHarness()

        const pending = client.request('textDocument/definition', {})

        expect(sent).toHaveLength(0)
        await expect(pending).rejects.toBeInstanceOf(LspCapabilityNotSupportedError)
    })

    test('서버가 해당 capability 를 선언하지 않으면 요청이 전송되지 않는다', async () => {
        const { sent, client } = createHarness()

        const initializePending = client.initialize({})
        const initializeRequest = sent[0] as JsonRpcRequest
        client.handleMessage(respond(initializeRequest.id as number, { capabilities: { hoverProvider: true } }))
        await initializePending

        expect(sent).toHaveLength(2)

        const pending = client.request('textDocument/definition', {})
        expect(sent).toHaveLength(2)
        await expect(pending).rejects.toBeInstanceOf(LspCapabilityNotSupportedError)
    })

    test('서버가 해당 capability 를 선언하면 요청이 정상 전송된다', async () => {
        const { sent, client } = createHarness()

        const initializePending = client.initialize({})
        const initializeRequest = sent[0] as JsonRpcRequest
        client.handleMessage(respond(initializeRequest.id as number, { capabilities: { definitionProvider: true } }))
        await initializePending

        const pending = client.request('textDocument/definition', { textDocument: { uri: 'file:///a.ts' } })
        expect(sent).toHaveLength(3)

        const request = sent[2] as JsonRpcRequest
        client.handleMessage(respond(request.id as number, null))
        await expect(pending).resolves.toBeNull()
    })

    test('capability map 에 없는 임의 메서드는 게이팅 없이 항상 전송된다', () => {
        const { sent, client } = createHarness()
        client.request('workspace/executeCommand', { command: 'custom' })
        expect(sent).toHaveLength(1)
    })
})

describe('createLspClient — 문서 동기화', () => {
    test('didOpen 은 textDocument/didOpen 알림을 보낸다', () => {
        const { sent, client } = createHarness()
        client.didOpen({ uri: 'file:///a.ts', languageId: 'typescript', version: 1, text: 'const a = 1' })

        expect(sent).toHaveLength(1)
        expect(sent[0]).toEqual({
            jsonrpc: '2.0',
            method: 'textDocument/didOpen',
            params: { textDocument: { uri: 'file:///a.ts', languageId: 'typescript', version: 1, text: 'const a = 1' } },
        })
    })

    test('didChange 는 버전을 증가시키며 incremental content change 를 보낸다', () => {
        const { sent, client } = createHarness()
        client.didOpen({ uri: 'file:///a.ts', languageId: 'typescript', version: 1, text: 'const a = 1' })

        client.didChange('file:///a.ts', [{ range: { start: { line: 0, character: 6 }, end: { line: 0, character: 7 } }, text: 'b' }])

        const notification = sent[1] as JsonRpcNotification
        expect(notification.method).toBe('textDocument/didChange')
        expect(notification.params).toEqual({
            textDocument: { uri: 'file:///a.ts', version: 2 },
            contentChanges: [{ range: { start: { line: 0, character: 6 }, end: { line: 0, character: 7 } }, text: 'b' }],
        })
    })

    test('열리지 않은 문서에 didChange 를 보내면 예외를 던진다', () => {
        const { client } = createHarness()
        expect(() =>
            client.didChange('file:///not-open.ts', [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, text: '' }]),
        ).toThrow(LspDocumentNotOpenError)
    })

    test('didClose 는 textDocument/didClose 알림을 보내고 이후 didChange 는 다시 예외를 던진다', () => {
        const { sent, client } = createHarness()
        client.didOpen({ uri: 'file:///a.ts', languageId: 'typescript', version: 1, text: 'x' })
        client.didClose('file:///a.ts')

        expect(sent[1]).toEqual({ jsonrpc: '2.0', method: 'textDocument/didClose', params: { textDocument: { uri: 'file:///a.ts' } } })
        expect(() => client.didChange('file:///a.ts', [])).toThrow(LspDocumentNotOpenError)
    })

    test('열리지 않은 문서를 didClose 해도 알림을 보내지 않는다', () => {
        const { sent, client } = createHarness()
        client.didClose('file:///never-opened.ts')
        expect(sent).toHaveLength(0)
    })
})

describe('createLspClient — dispose', () => {
    test('dispose 시 pending 요청을 모두 reject 한다', async () => {
        const { client } = createHarness()
        const pending = client.request('workspace/symbol', {})
        client.dispose()
        await expect(pending).rejects.toBeInstanceOf(Error)
    })
})
