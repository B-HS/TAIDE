import { describe, expect, test } from 'bun:test'
import type { CancellationToken, languages } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import { CODE_LENS_REFRESH_DEBOUNCE_MS, registerCodeLens, toMonacoCodeLens, toMonacoCommand } from '@shared/lib/lsp/adapters/code-lens'

const createFakeToken = (isCancellationRequested: boolean): CancellationToken => ({
    isCancellationRequested,
    onCancellationRequested: () => ({ dispose: () => {} }),
})

describe('toMonacoCommand', () => {
    test('LSP command 를 monaco Command(id/title/arguments) 로 변환한다', () => {
        expect(toMonacoCommand({ title: 'Go to Location', command: 'rust-analyzer.gotoLocation', arguments: ['file:///a.rs'] })).toEqual({
            id: 'rust-analyzer.gotoLocation',
            title: 'Go to Location',
            arguments: ['file:///a.rs'],
        })
    })

    test('command id 가 빈 문자열이면 undefined 를 반환한다 (미등록 커맨드 방어)', () => {
        expect(toMonacoCommand({ title: 'broken', command: '' })).toBeUndefined()
    })
})

describe('toMonacoCodeLens', () => {
    test('LSP range 를 1-based monaco range 로 변환하고 command 를 매핑한다', () => {
        const result = toMonacoCodeLens({
            range: { start: { line: 2, character: 0 }, end: { line: 2, character: 5 } },
            command: { title: '3 references', command: 'editor.action.showReferences', arguments: [] },
        })
        expect(result).toEqual({
            range: { startLineNumber: 3, startColumn: 1, endLineNumber: 3, endColumn: 6 },
            command: { id: 'editor.action.showReferences', title: '3 references', arguments: [] },
        })
    })

    test('command 가 없으면 undefined 로 남긴다 (resolve 대기 lens)', () => {
        const result = toMonacoCodeLens({ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } } })
        expect(result.command).toBeUndefined()
    })
})

type CodeLensProviderArg = Parameters<Monaco['languages']['registerCodeLensProvider']>[1]

const createTestLspClient = async (capabilities: ServerCapabilities, handleRequest: (method: string, params: unknown) => unknown) => {
    const client = createLspClient({
        send: (message) => {
            if (!isJsonRpcRequest(message)) return
            if (message.method === 'initialize') {
                client.handleMessage({ jsonrpc: '2.0', id: message.id, result: { capabilities } })
                return
            }
            client.handleMessage({ jsonrpc: '2.0', id: message.id, result: handleRequest(message.method, message.params) })
        },
        onNotification: () => {},
    })
    await client.initialize({})
    return client
}

class FakeEmitter {
    private listeners: (() => void)[] = []
    event = (listener: () => void) => {
        this.listeners.push(listener)
        return { dispose: () => (this.listeners = this.listeners.filter((entry) => entry !== listener)) }
    }
    fire() {
        this.listeners.forEach((listener) => listener())
    }
    dispose() {
        this.listeners = []
    }
}

const createFakeMonaco = () => {
    let capturedProvider: CodeLensProviderArg | undefined
    let disposeCallCount = 0
    const fakeMonaco = {
        Emitter: FakeEmitter,
        languages: {
            registerCodeLensProvider: (_languageId: string, provider: CodeLensProviderArg) => {
                capturedProvider = provider
                return { dispose: () => (disposeCallCount += 1) }
            },
        },
    }
    return { monaco: fakeMonaco as unknown as Monaco, getProvider: () => capturedProvider, getDisposeCallCount: () => disposeCallCount }
}

const fakeModel = { uri: { toString: () => 'file:///a.ts' } } as Parameters<languages.CodeLensProvider['provideCodeLenses']>[0]

describe('registerCodeLens', () => {
    test('서버가 codeLensProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        const disposable = registerCodeLens(monaco, client, 'typescript')
        expect(getProvider()).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('editorCodeLensEnabled 설정이 꺼지면 요청 없이 빈 렌즈를 반환한다', async () => {
        let requestCount = 0
        const client = await createTestLspClient({ codeLensProvider: {} }, () => {
            requestCount += 1
            return []
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerCodeLens(monaco, client, 'typescript', () => false)

        const result = await getProvider()?.provideCodeLenses(fakeModel, createFakeToken(false))
        expect(result).toEqual({ lenses: [] })
        expect(requestCount).toBe(0)
    })

    test('설정이 켜져 있으면 LSP codeLens 요청 결과를 monaco lens 로 변환한다', async () => {
        const client = await createTestLspClient({ codeLensProvider: {} }, () => [
            { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, command: { title: '1 reference', command: 'showRefs' } },
        ])
        const { monaco, getProvider } = createFakeMonaco()
        registerCodeLens(monaco, client, 'typescript')

        const result = await getProvider()?.provideCodeLenses(fakeModel, createFakeToken(false))
        expect(result?.lenses).toEqual([
            { range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 }, command: { id: 'showRefs', title: '1 reference' } },
        ])
    })

    test('요청 완료 후 토큰이 취소돼 있으면 결과를 폐기하고 빈 렌즈를 반환한다', async () => {
        const client = await createTestLspClient({ codeLensProvider: {} }, () => [
            { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } } },
        ])
        const { monaco, getProvider } = createFakeMonaco()
        registerCodeLens(monaco, client, 'typescript')

        const result = await getProvider()?.provideCodeLenses(fakeModel, createFakeToken(true))
        expect(result).toEqual({ lenses: [] })
    })

    test('resolveProvider 가 false 면 resolveCodeLens 를 노출하지 않는다', async () => {
        const client = await createTestLspClient({ codeLensProvider: { resolveProvider: false } }, () => [])
        const { monaco, getProvider } = createFakeMonaco()
        registerCodeLens(monaco, client, 'typescript')
        expect(getProvider()?.resolveCodeLens).toBeUndefined()
    })

    test('resolveCodeLens 는 provideCodeLenses 가 보관한 data 를 codeLens/resolve 요청에 실어 보낸다', async () => {
        const receivedParams: unknown[] = []
        const client = await createTestLspClient({ codeLensProvider: { resolveProvider: true } }, (method, params) => {
            if (method === 'textDocument/codeLens') {
                return [{ range: { start: { line: 4, character: 0 }, end: { line: 4, character: 1 } }, data: { kind: 'references' } }]
            }
            receivedParams.push(params)
            return { range: { start: { line: 4, character: 0 }, end: { line: 4, character: 1 } }, command: { title: '2 refs', command: 'showRefs' } }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerCodeLens(monaco, client, 'typescript')

        const provided = await getProvider()?.provideCodeLenses(fakeModel, createFakeToken(false))
        const unresolvedLens = provided?.lenses[0] as languages.CodeLens
        const resolved = await getProvider()?.resolveCodeLens?.(fakeModel, unresolvedLens, createFakeToken(false))

        expect(receivedParams).toEqual([
            { range: { start: { line: 4, character: 0 }, end: { line: 4, character: 1 } }, data: { kind: 'references' } },
        ])
        expect(resolved?.command).toEqual({ id: 'showRefs', title: '2 refs' })
    })

    test('resolveCodeLens 도중 취소되면 원본 codeLens 를 그대로 반환한다', async () => {
        const client = await createTestLspClient({ codeLensProvider: { resolveProvider: true } }, () => ({
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
            command: { title: 'late', command: 'lateCommand' },
        }))
        const { monaco, getProvider } = createFakeMonaco()
        registerCodeLens(monaco, client, 'typescript')

        const unresolvedLens: languages.CodeLens = { range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } }
        const resolved = await getProvider()?.resolveCodeLens?.(fakeModel, unresolvedLens, createFakeToken(true))
        expect(resolved).toBe(unresolvedLens)
    })

    test('workspace/codeLens/refresh 수신 시 디바운스 후 onDidChange 를 발화한다', async () => {
        const client = await createTestLspClient({ codeLensProvider: {} }, () => [])
        const { monaco, getProvider } = createFakeMonaco()
        registerCodeLens(monaco, client, 'typescript')

        let fired = false
        getProvider()?.onDidChange?.(() => (fired = true))

        client.handleMessage({ jsonrpc: '2.0', id: 'server-1', method: 'workspace/codeLens/refresh' })

        expect(fired).toBe(false)
        await new Promise((resolve) => setTimeout(resolve, CODE_LENS_REFRESH_DEBOUNCE_MS + 50))
        expect(fired).toBe(true)
    })

    test('dispose 는 provider 등록과 refresh 구독을 모두 해제한다', async () => {
        const client = await createTestLspClient({ codeLensProvider: {} }, () => [])
        const { monaco, getProvider, getDisposeCallCount } = createFakeMonaco()
        const disposable = registerCodeLens(monaco, client, 'typescript')

        let fired = false
        getProvider()?.onDidChange?.(() => (fired = true))
        disposable.dispose()
        expect(getDisposeCallCount()).toBe(1)

        client.handleMessage({ jsonrpc: '2.0', id: 'server-2', method: 'workspace/codeLens/refresh' })
        await new Promise((resolve) => setTimeout(resolve, CODE_LENS_REFRESH_DEBOUNCE_MS + 50))
        expect(fired).toBe(false)
    })
})
