import { afterEach, describe, expect, test } from 'bun:test'
import type { CancellationToken } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { resetPeekModelPreloadStateForTests } from '@shared/lib/lsp/peek-model-preload'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import { registerReferences } from '@shared/lib/lsp/adapters/references'

/**
 * `registerReferences` preloads target-file models with a real 60s TTL (`peek-model-preload.ts`)
 * that never expires within a test run — left unreset, a path this file preloads would leak into
 * every other test *file* that runs afterward (bun does not isolate modules per file).
 */
afterEach(() => {
    resetPeekModelPreloadStateForTests()
})

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

const uriCache = new Map<string, { scheme: string; fsPath: string; toString: () => string }>()

const createFakeUri = (raw: string) => {
    const schemeMatch = /^([a-zA-Z][a-zA-Z\d+.-]*):\/\/(.*)$/.exec(raw)
    const scheme = schemeMatch ? schemeMatch[1] : 'file'
    const fsPath = schemeMatch ? schemeMatch[2] : raw
    const canonical = `${scheme}://${fsPath}`
    const cached = uriCache.get(canonical)
    if (cached) return cached
    const uri = { scheme, fsPath, toString: () => canonical }
    uriCache.set(canonical, uri)
    return uri
}

type FakeModel = {
    content: string
    languageId: string
    uri: ReturnType<typeof createFakeUri>
    disposed: boolean
    dispose: () => void
    isDisposed: () => boolean
}
type ReferenceProviderArg = Parameters<Monaco['languages']['registerReferenceProvider']>[1]

const createFakeMonaco = () => {
    const models = new Map<string, FakeModel>()
    let captured: ReferenceProviderArg | undefined

    const fakeMonaco = {
        Uri: { parse: createFakeUri, file: (path: string) => createFakeUri(`file://${path}`) },
        editor: {
            getModel: (uri: { toString: () => string }) => models.get(uri.toString()) ?? null,
            createModel: (content: string, languageId: string, uri: ReturnType<typeof createFakeUri>) => {
                const model: FakeModel = {
                    content,
                    languageId,
                    uri,
                    disposed: false,
                    dispose: () => (model.disposed = true),
                    isDisposed: () => model.disposed,
                }
                models.set(uri.toString(), model)
                return model
            },
            getEditors: () => [] as { getModel: () => FakeModel | undefined }[],
        },
        languages: {
            registerReferenceProvider: (_languageId: string, provider: ReferenceProviderArg) => {
                captured = provider
                return { dispose: () => {} }
            },
        },
    }

    return {
        monaco: fakeMonaco as unknown as Monaco,
        getProvider: () => captured,
        hasModelFor: (path: string) => models.has(`file://${path}`),
    }
}

const fakeModel = { uri: { toString: () => 'file:///workspace/source.ts' } } as Parameters<ReferenceProviderArg['provideReferences']>[0]
const fakePosition = { lineNumber: 1, column: 1 } as Parameters<ReferenceProviderArg['provideReferences']>[1]
const fakeContext = { includeDeclaration: true } as Parameters<ReferenceProviderArg['provideReferences']>[2]

describe('registerReferences — capability 게이트', () => {
    test('referencesProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, getProvider } = createFakeMonaco()

        const disposable = registerReferences(monaco, client, 'typescript')

        expect(getProvider()).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })
})

describe('registerReferences — 위치 정규화·프리로드·취소', () => {
    test('Location[] 결과를 monaco Location[] 으로 정규화한다', async () => {
        const client = await createTestLspClient({ referencesProvider: true }, () => [
            { uri: 'file:///workspace/a.ts', range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } } },
            { uri: 'file:///workspace/b.ts', range: { start: { line: 2, character: 3 }, end: { line: 2, character: 6 } } },
        ])
        const { monaco, getProvider } = createFakeMonaco()
        registerReferences(monaco, client, 'typescript')

        const result = await getProvider()?.provideReferences(fakeModel, fakePosition, fakeContext, createFakeToken(false))

        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/a.ts'), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } },
            { uri: monaco.Uri.parse('file:///workspace/b.ts'), range: { startLineNumber: 3, startColumn: 4, endLineNumber: 3, endColumn: 7 } },
        ])
    })

    test('결과가 없으면 빈 배열을 반환한다', async () => {
        const client = await createTestLspClient({ referencesProvider: true }, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        registerReferences(monaco, client, 'typescript')

        const result = await getProvider()?.provideReferences(fakeModel, fakePosition, fakeContext, createFakeToken(false))

        expect(result).toEqual([])
    })

    test('응답 후 토큰이 취소돼 있으면 빈 배열을 반환하고 모델을 프리로드하지 않는다', async () => {
        const client = await createTestLspClient({ referencesProvider: true }, () => [
            { uri: 'file:///workspace/a.ts', range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } } },
        ])
        const { monaco, getProvider, hasModelFor } = createFakeMonaco()
        registerReferences(monaco, client, 'typescript')

        const result = await getProvider()?.provideReferences(fakeModel, fakePosition, fakeContext, createFakeToken(true))

        expect(result).toEqual([])
        expect(hasModelFor('/workspace/a.ts')).toBe(false)
    })

    test('대상 파일 모델이 이미 있으면 프리로드 없이 그대로 응답한다', async () => {
        const client = await createTestLspClient({ referencesProvider: true }, () => [
            { uri: 'file:///workspace/a.ts', range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } } },
        ])
        const { monaco, getProvider } = createFakeMonaco()
        monaco.editor.createModel('existing', 'typescript', monaco.Uri.file('/workspace/a.ts'))
        registerReferences(monaco, client, 'typescript')

        const result = await getProvider()?.provideReferences(fakeModel, fakePosition, fakeContext, createFakeToken(false))

        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/a.ts'), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } },
        ])
    })

    test('includeDeclaration 를 그대로 요청 context 에 실어 보낸다', async () => {
        const receivedParams: unknown[] = []
        const client = await createTestLspClient({ referencesProvider: true }, (_method, params) => {
            receivedParams.push(params)
            return []
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerReferences(monaco, client, 'typescript')

        await getProvider()?.provideReferences(
            fakeModel,
            fakePosition,
            { includeDeclaration: false } as Parameters<ReferenceProviderArg['provideReferences']>[2],
            createFakeToken(false),
        )

        expect(receivedParams).toEqual([
            { textDocument: { uri: 'file:///workspace/source.ts' }, position: { line: 0, character: 0 }, context: { includeDeclaration: false } },
        ])
    })
})
