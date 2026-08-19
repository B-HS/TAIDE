import { describe, expect, test } from 'bun:test'
import type { JsonRpcErrorResponse, JsonRpcNotification, JsonRpcRequest, ServerCapabilities } from '@shared/lib/lsp/protocol'
import { LspCapabilityNotSupportedError, LspDocumentNotOpenError, createLspClient } from '@shared/lib/lsp/client'
import type { OutgoingMessage } from '@shared/lib/lsp/client'
import { registerServerRequestHandler } from '@shared/lib/lsp/server-request-handler-registry'

const createHarness = () => {
    const sent: OutgoingMessage[] = []
    const notifications: JsonRpcNotification[] = []
    const client = createLspClient({
        send: (message) => sent.push(message),
        onNotification: (notification) => notifications.push(notification),
    })
    return { sent, notifications, client }
}

const respond = (id: number, result: unknown) => ({ jsonrpc: '2.0', id, result })

const flushMicrotasks = () => new Promise<void>((resolve) => setTimeout(resolve, 0))

describe('createLspClient — request/response id 매칭', () => {
    test('요청에 발급한 id 와 응답의 id 를 매칭해 resolve 한다', async () => {
        const { sent, client } = createHarness()

        const pending = client.request<{ ok: boolean }>('a/method', { query: 'foo' })
        expect(sent).toHaveLength(1)
        const request = sent[0] as JsonRpcRequest
        expect(request.method).toBe('a/method')

        client.handleMessage(respond(request.id as number, { ok: true }))

        await expect(pending).resolves.toEqual({ ok: true })
    })

    test('id 가 일치하지 않는 응답은 무시된다', async () => {
        const { sent, client } = createHarness()

        const pending = client.request('a/method', {})
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

        const pending = client.request('a/method', {})
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
        client.request('a/method', { query: 'foo' })
        expect(sent).toHaveLength(1)
    })
})

describe('createLspClient — 확장 capability 게이트', () => {
    const initializeWithCapabilities = async (
        client: ReturnType<typeof createLspClient>,
        sent: OutgoingMessage[],
        capabilities: ServerCapabilities,
    ) => {
        const initializePending = client.initialize({})
        const initializeRequest = sent[0] as JsonRpcRequest
        client.handleMessage(respond(initializeRequest.id as number, { capabilities }))
        await initializePending
    }

    const capabilityCases: { method: string; capabilities: ServerCapabilities }[] = [
        { method: 'textDocument/codeAction', capabilities: { codeActionProvider: true } },
        { method: 'textDocument/codeLens', capabilities: { codeLensProvider: {} } },
        { method: 'codeLens/resolve', capabilities: { codeLensProvider: { resolveProvider: true } } },
        { method: 'textDocument/foldingRange', capabilities: { foldingRangeProvider: true } },
        { method: 'textDocument/implementation', capabilities: { implementationProvider: true } },
        { method: 'textDocument/typeDefinition', capabilities: { typeDefinitionProvider: true } },
        { method: 'textDocument/declaration', capabilities: { declarationProvider: true } },
        { method: 'workspace/symbol', capabilities: { workspaceSymbolProvider: true } },
        { method: 'workspace/executeCommand', capabilities: { executeCommandProvider: {} } },
        { method: 'textDocument/rangeFormatting', capabilities: { documentRangeFormattingProvider: true } },
        { method: 'textDocument/onTypeFormatting', capabilities: { documentOnTypeFormattingProvider: { firstTriggerCharacter: ';' } } },
        {
            method: 'textDocument/semanticTokens/full',
            capabilities: { semanticTokensProvider: { legend: { tokenTypes: [], tokenModifiers: [] }, full: true } },
        },
    ]

    for (const { method, capabilities } of capabilityCases) {
        test(`${method} 는 해당 capability 가 선언되면 전송된다`, async () => {
            const { sent, client } = createHarness()
            await initializeWithCapabilities(client, sent, capabilities)

            client.request(method, {})
            expect(sent).toHaveLength(3)
        })

        test(`${method} 는 capability 가 선언되지 않으면 게이팅된다`, async () => {
            const { sent, client } = createHarness()
            await initializeWithCapabilities(client, sent, {})

            const pending = client.request(method, {})
            expect(sent).toHaveLength(2)
            await expect(pending).rejects.toBeInstanceOf(LspCapabilityNotSupportedError)
        })
    }

    test('codeAction/resolve 와 codeLens/resolve 는 resolveProvider 가 없으면 게이팅된다', async () => {
        const { sent, client } = createHarness()
        await initializeWithCapabilities(client, sent, { codeActionProvider: true, codeLensProvider: {} })

        const codeActionResolvePending = client.request('codeAction/resolve', {})
        const codeLensResolvePending = client.request('codeLens/resolve', {})
        expect(sent).toHaveLength(2)
        await expect(codeActionResolvePending).rejects.toBeInstanceOf(LspCapabilityNotSupportedError)
        await expect(codeLensResolvePending).rejects.toBeInstanceOf(LspCapabilityNotSupportedError)
    })

    test('codeAction/resolve 는 codeActionProvider.resolveProvider 가 true 면 전송된다', async () => {
        const { sent, client } = createHarness()
        await initializeWithCapabilities(client, sent, { codeActionProvider: { resolveProvider: true } })

        client.request('codeAction/resolve', {})
        expect(sent).toHaveLength(3)
    })

    test('textDocument/semanticTokens/full/delta 는 semanticTokensProvider.full 이 boolean 이면 게이팅된다', async () => {
        const { sent, client } = createHarness()
        await initializeWithCapabilities(client, sent, {
            semanticTokensProvider: { legend: { tokenTypes: [], tokenModifiers: [] }, full: true },
        })

        const pending = client.request('textDocument/semanticTokens/full/delta', {})
        expect(sent).toHaveLength(2)
        await expect(pending).rejects.toBeInstanceOf(LspCapabilityNotSupportedError)
    })

    test('textDocument/semanticTokens/full/delta 는 full.delta 가 true 가 아니면 게이팅된다', async () => {
        const { sent, client } = createHarness()
        await initializeWithCapabilities(client, sent, {
            semanticTokensProvider: { legend: { tokenTypes: [], tokenModifiers: [] }, full: { delta: false } },
        })

        const pending = client.request('textDocument/semanticTokens/full/delta', {})
        expect(sent).toHaveLength(2)
        await expect(pending).rejects.toBeInstanceOf(LspCapabilityNotSupportedError)
    })

    test('textDocument/semanticTokens/full/delta 는 full.delta 가 true 면 전송된다', async () => {
        const { sent, client } = createHarness()
        await initializeWithCapabilities(client, sent, {
            semanticTokensProvider: { legend: { tokenTypes: [], tokenModifiers: [] }, full: { delta: true } },
        })

        client.request('textDocument/semanticTokens/full/delta', {})
        expect(sent).toHaveLength(3)
    })
})

describe('createLspClient — 서버→클라이언트 요청 라우팅', () => {
    test('등록된 핸들러의 결과를 JSON-RPC 응답으로 돌려보낸다', async () => {
        const { sent, client } = createHarness()
        const dispose = registerServerRequestHandler('test/echo', async (params) => ({ echoed: params }))

        client.handleMessage({ jsonrpc: '2.0', id: 'req-1', method: 'test/echo', params: { value: 1 } })
        await flushMicrotasks()

        expect(sent).toEqual([{ jsonrpc: '2.0', id: 'req-1', result: { echoed: { value: 1 } } }])
        dispose()
    })

    test('등록되지 않은 메서드는 -32601 MethodNotFound 에러 응답을 보낸다', async () => {
        const { sent, client } = createHarness()

        client.handleMessage({ jsonrpc: '2.0', id: 7, method: 'test/unregistered-method', params: {} })
        await flushMicrotasks()

        expect(sent).toHaveLength(1)
        const response = sent[0] as JsonRpcErrorResponse
        expect(response.id).toBe(7)
        expect(response.error.code).toBe(-32601)
    })

    test('핸들러가 실패하면 내부 에러 응답을 보낸다', async () => {
        const { sent, client } = createHarness()
        const dispose = registerServerRequestHandler('test/throwing', async () => {
            throw new Error('boom')
        })

        client.handleMessage({ jsonrpc: '2.0', id: 3, method: 'test/throwing', params: {} })
        await flushMicrotasks()

        expect(sent).toHaveLength(1)
        const response = sent[0] as JsonRpcErrorResponse
        expect(response.error.code).toBe(-32603)
        expect(response.error.message).toBe('boom')
        dispose()
    })

    test('핸들러 등록을 해제하면 다시 -32601 로 응답한다', async () => {
        const { sent, client } = createHarness()
        const dispose = registerServerRequestHandler('test/once', async () => null)
        dispose()

        client.handleMessage({ jsonrpc: '2.0', id: 9, method: 'test/once', params: {} })
        await flushMicrotasks()

        expect(sent).toHaveLength(1)
        const response = sent[0] as JsonRpcErrorResponse
        expect(response.error.code).toBe(-32601)
    })

    test('registerRequestHandler 로 등록한 인스턴스 핸들러가 전역 레지스트리보다 우선한다', async () => {
        const { sent, client } = createHarness()
        const globalDispose = registerServerRequestHandler('test/scoped', async () => ({ from: 'global' }))
        const instanceDispose = client.registerRequestHandler('test/scoped', async () => ({ from: 'instance' }))

        client.handleMessage({ jsonrpc: '2.0', id: 'scoped-1', method: 'test/scoped', params: {} })
        await flushMicrotasks()

        expect(sent).toEqual([{ jsonrpc: '2.0', id: 'scoped-1', result: { from: 'instance' } }])
        instanceDispose()
        globalDispose()
    })

    test('registerRequestHandler 는 다른 클라이언트 인스턴스에 영향을 주지 않는다 (세션별 격리)', async () => {
        const first = createHarness()
        const second = createHarness()
        first.client.registerRequestHandler('test/session-scoped', async () => ({ from: 'first' }))

        second.client.handleMessage({ jsonrpc: '2.0', id: 'sess-1', method: 'test/session-scoped', params: {} })
        await flushMicrotasks()

        expect(second.sent).toHaveLength(1)
        const response = second.sent[0] as JsonRpcErrorResponse
        expect(response.error.code).toBe(-32601)
    })

    test('인스턴스 핸들러를 해제하면 전역 레지스트리로 폴백한다', async () => {
        const { sent, client } = createHarness()
        const globalDispose = registerServerRequestHandler('test/fallback', async () => ({ from: 'global' }))
        const instanceDispose = client.registerRequestHandler('test/fallback', async () => ({ from: 'instance' }))
        instanceDispose()

        client.handleMessage({ jsonrpc: '2.0', id: 'fallback-1', method: 'test/fallback', params: {} })
        await flushMicrotasks()

        expect(sent).toEqual([{ jsonrpc: '2.0', id: 'fallback-1', result: { from: 'global' } }])
        globalDispose()
    })

    test('서버 요청 처리는 pending 클라이언트 요청 매칭에 영향을 주지 않는다', async () => {
        const { sent, client } = createHarness()
        const dispose = registerServerRequestHandler('test/interleaved', async () => [null])

        const pending = client.request('a/method', { query: 'foo' })
        const clientRequest = sent[0] as JsonRpcRequest

        client.handleMessage({ jsonrpc: '2.0', id: 'server-1', method: 'test/interleaved', params: { items: [{}] } })
        await flushMicrotasks()

        client.handleMessage(respond(clientRequest.id as number, { ok: true }))
        await expect(pending).resolves.toEqual({ ok: true })

        expect(sent).toContainEqual({ jsonrpc: '2.0', id: 'server-1', result: [null] })
        dispose()
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

    test('열린 문서에 didSave 를 호출하면 textDocument/didSave 알림을 보낸다', () => {
        const { sent, client } = createHarness()
        client.didOpen({ uri: 'file:///a.ts', languageId: 'typescript', version: 1, text: 'x' })
        client.didSave('file:///a.ts')

        expect(sent[1]).toEqual({ jsonrpc: '2.0', method: 'textDocument/didSave', params: { textDocument: { uri: 'file:///a.ts' } } })
    })

    test('열리지 않은 문서에 didSave 를 호출해도 알림을 보내지 않는다', () => {
        const { sent, client } = createHarness()
        client.didSave('file:///never-opened.ts')
        expect(sent).toHaveLength(0)
    })

    test('didClose 이후에는 같은 uri 로 didSave 를 호출해도 알림을 보내지 않는다', () => {
        const { sent, client } = createHarness()
        client.didOpen({ uri: 'file:///a.ts', languageId: 'typescript', version: 1, text: 'x' })
        client.didClose('file:///a.ts')
        client.didSave('file:///a.ts')

        expect(sent).toHaveLength(2)
    })

    test('getDocumentVersion 은 열린 문서의 현재 버전을 반환하고, 열리지 않은 문서는 undefined 를 반환한다', () => {
        const { client } = createHarness()
        client.didOpen({ uri: 'file:///a.ts', languageId: 'typescript', version: 1, text: 'x' })
        client.didChange('file:///a.ts', [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, text: 'y' }])

        expect(client.getDocumentVersion('file:///a.ts')).toBe(2)
        expect(client.getDocumentVersion('file:///never-opened.ts')).toBeUndefined()
    })
})

describe('createLspClient — dispose', () => {
    test('dispose 시 pending 요청을 모두 reject 한다', async () => {
        const { client } = createHarness()
        const pending = client.request('a/method', {})
        client.dispose()
        await expect(pending).rejects.toBeInstanceOf(Error)
    })
})

describe('createLspClient — rejectPendingRequests (R7#1 재핸드셰이크 준비)', () => {
    test('대기 중인 요청을 모두 지정한 사유로 reject 하고, 이후 같은 id 로 오는 응답은 무시한다', async () => {
        const { sent, client } = createHarness()
        const pending = client.request('a/method', {})

        client.rejectPendingRequests(new Error('lsp session reinitializing after crash'))

        await expect(pending).rejects.toThrow('lsp session reinitializing after crash')

        const requestId = (sent[0] as JsonRpcRequest).id
        expect(() => client.handleMessage(respond(requestId as number, { ok: true }))).not.toThrow()
    })

    test('요청 핸들러 등록·구독 상태는 유지한다 (dispose 와 달리 세션 정체성은 보존)', async () => {
        const { client } = createHarness()
        const disposeHandler = registerServerRequestHandler('test/rejectPendingRequests-keep-handler', async () => 'still-here')
        const notificationDisposable = client.onDiagnostics(() => {})

        client.rejectPendingRequests(new Error('reinitializing'))

        client.handleMessage({ jsonrpc: '2.0', id: 'req-1', method: 'test/rejectPendingRequests-keep-handler' } as JsonRpcRequest)
        await flushMicrotasks()

        disposeHandler()
        notificationDisposable.dispose()
    })

    test('새 요청은 재초기화 이후에도 정상적으로 매칭된다', async () => {
        const { sent, client } = createHarness()
        client.rejectPendingRequests(new Error('reinitializing'))

        const pending = client.request<{ ok: boolean }>('b/method', {})
        const requestId = (sent.at(-1) as JsonRpcRequest).id
        client.handleMessage(respond(requestId as number, { ok: true }))

        await expect(pending).resolves.toEqual({ ok: true })
    })
})
