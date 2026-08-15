import { describe, expect, test } from 'bun:test'
import type { CancellationToken, languages } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import { registerFormatting, registerOnTypeFormatting, registerRangeFormatting } from '@shared/lib/lsp/adapters/formatting'

const createFakeToken = (isCancellationRequested: boolean): CancellationToken => ({
    isCancellationRequested,
    onCancellationRequested: () => ({ dispose: () => {} }),
})

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

type CapturedProviders = {
    formatting?: Parameters<Monaco['languages']['registerDocumentFormattingEditProvider']>[1]
    rangeFormatting?: Parameters<Monaco['languages']['registerDocumentRangeFormattingEditProvider']>[1]
    onTypeFormatting?: Parameters<Monaco['languages']['registerOnTypeFormattingEditProvider']>[1]
}

const createFakeMonaco = () => {
    const captured: CapturedProviders = {}
    let disposeCallCount = 0
    const fakeMonaco = {
        languages: {
            registerDocumentFormattingEditProvider: (_languageId: string, provider: CapturedProviders['formatting']) => {
                captured.formatting = provider
                return { dispose: () => (disposeCallCount += 1) }
            },
            registerDocumentRangeFormattingEditProvider: (_languageId: string, provider: CapturedProviders['rangeFormatting']) => {
                captured.rangeFormatting = provider
                return { dispose: () => (disposeCallCount += 1) }
            },
            registerOnTypeFormattingEditProvider: (_languageId: string, provider: CapturedProviders['onTypeFormatting']) => {
                captured.onTypeFormatting = provider
                return { dispose: () => (disposeCallCount += 1) }
            },
        },
    }
    return { monaco: fakeMonaco as unknown as Monaco, captured, getDisposeCallCount: () => disposeCallCount }
}

const fakeModel = { uri: { toString: () => 'file:///a.ts' } } as Parameters<
    languages.DocumentFormattingEditProvider['provideDocumentFormattingEdits']
>[0]
const fakeOptions = { tabSize: 4, insertSpaces: true } as Parameters<languages.DocumentFormattingEditProvider['provideDocumentFormattingEdits']>[1]
const fakeRange = { startLineNumber: 2, startColumn: 1, endLineNumber: 4, endColumn: 1 } as Parameters<
    languages.DocumentRangeFormattingEditProvider['provideDocumentRangeFormattingEdits']
>[1]
const fakePosition = { lineNumber: 3, column: 5 } as Parameters<languages.OnTypeFormattingEditProvider['provideOnTypeFormattingEdits']>[1]

describe('registerFormatting', () => {
    test('서버가 documentFormattingProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, captured } = createFakeMonaco()
        const disposable = registerFormatting(monaco, client, 'typescript')
        expect(captured.formatting).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('LSP textDocument/formatting 응답을 monaco TextEdit 로 변환한다', async () => {
        const client = await createTestLspClient({ documentFormattingProvider: true }, () => [
            { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 3 } }, newText: 'foo' },
        ])
        const { monaco, captured } = createFakeMonaco()
        registerFormatting(monaco, client, 'typescript')

        const result = await captured.formatting?.provideDocumentFormattingEdits(fakeModel, fakeOptions, createFakeToken(false))
        expect(result).toEqual([{ range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 4 }, text: 'foo' }])
    })

    test('취소된 토큰이면 서버 응답을 폐기하고 빈 배열을 반환한다', async () => {
        const client = await createTestLspClient({ documentFormattingProvider: true }, () => [
            { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' },
        ])
        const { monaco, captured } = createFakeMonaco()
        registerFormatting(monaco, client, 'typescript')

        const result = await captured.formatting?.provideDocumentFormattingEdits(fakeModel, fakeOptions, createFakeToken(true))
        expect(result).toEqual([])
    })
})

describe('registerRangeFormatting', () => {
    test('서버가 documentRangeFormattingProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, captured } = createFakeMonaco()
        const disposable = registerRangeFormatting(monaco, client, 'typescript')
        expect(captured.rangeFormatting).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('monaco range 를 LSP range 로 변환해 textDocument/rangeFormatting 요청을 보낸다', async () => {
        const receivedParams: unknown[] = []
        const client = await createTestLspClient({ documentRangeFormattingProvider: true }, (_method, params) => {
            receivedParams.push(params)
            return [{ range: { start: { line: 1, character: 0 }, end: { line: 3, character: 0 } }, newText: 'formatted' }]
        })
        const { monaco, captured } = createFakeMonaco()
        registerRangeFormatting(monaco, client, 'typescript')

        const result = await captured.rangeFormatting?.provideDocumentRangeFormattingEdits(fakeModel, fakeRange, fakeOptions, createFakeToken(false))

        expect(receivedParams).toEqual([
            {
                textDocument: { uri: 'file:///a.ts' },
                range: { start: { line: 1, character: 0 }, end: { line: 3, character: 0 } },
                options: { tabSize: 4, insertSpaces: true },
            },
        ])
        expect(result).toEqual([{ range: { startLineNumber: 2, startColumn: 1, endLineNumber: 4, endColumn: 1 }, text: 'formatted' }])
    })

    test('취소된 토큰이면 서버 응답을 폐기하고 빈 배열을 반환한다', async () => {
        const client = await createTestLspClient({ documentRangeFormattingProvider: true }, () => [
            { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' },
        ])
        const { monaco, captured } = createFakeMonaco()
        registerRangeFormatting(monaco, client, 'typescript')

        const result = await captured.rangeFormatting?.provideDocumentRangeFormattingEdits(fakeModel, fakeRange, fakeOptions, createFakeToken(true))
        expect(result).toEqual([])
    })
})

describe('registerOnTypeFormatting', () => {
    test('서버가 documentOnTypeFormattingProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, captured } = createFakeMonaco()
        const disposable = registerOnTypeFormatting(monaco, client, 'typescript')
        expect(captured.onTypeFormatting).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('moreTriggerCharacter 가 없으면 firstTriggerCharacter 하나만 트리거 문자로 등록한다', async () => {
        const client = await createTestLspClient({ documentOnTypeFormattingProvider: { firstTriggerCharacter: '}' } }, () => null)
        const { monaco, captured } = createFakeMonaco()
        registerOnTypeFormatting(monaco, client, 'typescript')

        expect(captured.onTypeFormatting?.autoFormatTriggerCharacters).toEqual(['}'])
    })

    test('firstTriggerCharacter 뒤에 moreTriggerCharacter 를 그대로 이어붙인다 (서버 선언 순서 보존)', async () => {
        const client = await createTestLspClient(
            { documentOnTypeFormattingProvider: { firstTriggerCharacter: ';', moreTriggerCharacter: ['}', '\n'] } },
            () => null,
        )
        const { monaco, captured } = createFakeMonaco()
        registerOnTypeFormatting(monaco, client, 'typescript')

        expect(captured.onTypeFormatting?.autoFormatTriggerCharacters).toEqual([';', '}', '\n'])
    })

    test('monaco position 과 입력 문자를 LSP textDocument/onTypeFormatting 요청으로 보낸다', async () => {
        const receivedParams: unknown[] = []
        const client = await createTestLspClient({ documentOnTypeFormattingProvider: { firstTriggerCharacter: '}' } }, (_method, params) => {
            receivedParams.push(params)
            return [{ range: { start: { line: 2, character: 0 }, end: { line: 2, character: 1 } }, newText: '  }' }]
        })
        const { monaco, captured } = createFakeMonaco()
        registerOnTypeFormatting(monaco, client, 'typescript')

        const result = await captured.onTypeFormatting?.provideOnTypeFormattingEdits(
            fakeModel,
            fakePosition,
            '}',
            fakeOptions,
            createFakeToken(false),
        )

        expect(receivedParams).toEqual([
            {
                textDocument: { uri: 'file:///a.ts' },
                position: { line: 2, character: 4 },
                ch: '}',
                options: { tabSize: 4, insertSpaces: true },
            },
        ])
        expect(result).toEqual([{ range: { startLineNumber: 3, startColumn: 1, endLineNumber: 3, endColumn: 2 }, text: '  }' }])
    })

    test('취소된 토큰이면 서버 응답을 폐기하고 빈 배열을 반환한다', async () => {
        const client = await createTestLspClient({ documentOnTypeFormattingProvider: { firstTriggerCharacter: '}' } }, () => [
            { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' },
        ])
        const { monaco, captured } = createFakeMonaco()
        registerOnTypeFormatting(monaco, client, 'typescript')

        const result = await captured.onTypeFormatting?.provideOnTypeFormattingEdits(fakeModel, fakePosition, '}', fakeOptions, createFakeToken(true))
        expect(result).toEqual([])
    })

    test('dispose 는 provider 등록을 해제한다', async () => {
        const client = await createTestLspClient({ documentOnTypeFormattingProvider: { firstTriggerCharacter: '}' } }, () => null)
        const { monaco, getDisposeCallCount } = createFakeMonaco()
        const disposable = registerOnTypeFormatting(monaco, client, 'typescript')
        disposable.dispose()
        expect(getDisposeCallCount()).toBe(1)
    })
})
