import { afterEach, describe, expect, test } from 'bun:test'
import type { CancellationToken } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { resetPeekModelPreloadStateForTests } from '@shared/lib/lsp/peek-model-preload'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import { registerDeclaration } from '@shared/lib/lsp/adapters/declaration'
import { registerDefinition } from '@shared/lib/lsp/adapters/definition'
import { registerImplementation } from '@shared/lib/lsp/adapters/implementation'
import { registerTypeDefinition } from '@shared/lib/lsp/adapters/type-definition'

/**
 * `registerDefinition`/etc. preload target-file models with a real 60s TTL (`peek-model-preload.ts`)
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
type CapturedProviders = {
    definition?: Parameters<Monaco['languages']['registerDefinitionProvider']>[1]
    implementation?: Parameters<Monaco['languages']['registerImplementationProvider']>[1]
    typeDefinition?: Parameters<Monaco['languages']['registerTypeDefinitionProvider']>[1]
    declaration?: Parameters<Monaco['languages']['registerDeclarationProvider']>[1]
}

const createFakeMonaco = () => {
    const models = new Map<string, FakeModel>()
    const captured: CapturedProviders = {}

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
            registerDefinitionProvider: (_languageId: string, provider: CapturedProviders['definition']) => {
                captured.definition = provider
                return { dispose: () => {} }
            },
            registerImplementationProvider: (_languageId: string, provider: CapturedProviders['implementation']) => {
                captured.implementation = provider
                return { dispose: () => {} }
            },
            registerTypeDefinitionProvider: (_languageId: string, provider: CapturedProviders['typeDefinition']) => {
                captured.typeDefinition = provider
                return { dispose: () => {} }
            },
            registerDeclarationProvider: (_languageId: string, provider: CapturedProviders['declaration']) => {
                captured.declaration = provider
                return { dispose: () => {} }
            },
        },
    }

    return { monaco: fakeMonaco as unknown as Monaco, captured, hasModelFor: (path: string) => models.has(`file://${path}`) }
}

const fakeModel = { uri: { toString: () => 'file:///workspace/source.ts' } } as Parameters<
    Parameters<Monaco['languages']['registerDefinitionProvider']>[1]['provideDefinition']
>[0]
const fakePosition = { lineNumber: 1, column: 1 } as Parameters<
    Parameters<Monaco['languages']['registerDefinitionProvider']>[1]['provideDefinition']
>[1]

describe('registerDefinition — capability 게이트', () => {
    test('definitionProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, captured } = createFakeMonaco()

        const disposable = registerDefinition(monaco, client, 'typescript')

        expect(captured.definition).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })
})

describe('registerDefinition — 위치 정규화·프리로드·취소', () => {
    test('단일 Location 결과를 monaco Location 으로 정규화한다', async () => {
        const client = await createTestLspClient({ definitionProvider: true }, () => ({
            uri: 'file:///workspace/target.ts',
            range: { start: { line: 4, character: 0 }, end: { line: 4, character: 3 } },
        }))
        const { monaco, captured } = createFakeMonaco()
        registerDefinition(monaco, client, 'typescript')

        const result = await captured.definition?.provideDefinition(fakeModel, fakePosition, createFakeToken(false))

        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/target.ts'), range: { startLineNumber: 5, startColumn: 1, endLineNumber: 5, endColumn: 4 } },
        ])
    })

    test('LocationLink[] 결과는 targetUri/targetRange 로 정규화한다', async () => {
        const client = await createTestLspClient({ definitionProvider: true }, () => [
            {
                targetUri: 'file:///workspace/target.ts',
                targetRange: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
                targetSelectionRange: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
            },
        ])
        const { monaco, captured } = createFakeMonaco()
        registerDefinition(monaco, client, 'typescript')

        const result = await captured.definition?.provideDefinition(fakeModel, fakePosition, createFakeToken(false))

        expect(result).toEqual([
            {
                uri: monaco.Uri.parse('file:///workspace/target.ts'),
                range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 },
                targetSelectionRange: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 },
            },
        ])
    })

    test('LocationLink[] 은 targetSelectionRange 를 보존한다 (targetRange 는 문서 주석을 포함해 더 넓을 수 있다)', async () => {
        const client = await createTestLspClient({ definitionProvider: true }, () => [
            {
                targetUri: 'file:///workspace/target.ts',
                targetRange: { start: { line: 0, character: 0 }, end: { line: 5, character: 1 } },
                targetSelectionRange: { start: { line: 3, character: 4 }, end: { line: 3, character: 10 } },
            },
        ])
        const { monaco, captured } = createFakeMonaco()
        registerDefinition(monaco, client, 'typescript')

        const result = await captured.definition?.provideDefinition(fakeModel, fakePosition, createFakeToken(false))

        expect(result).toEqual([
            {
                uri: monaco.Uri.parse('file:///workspace/target.ts'),
                range: { startLineNumber: 1, startColumn: 1, endLineNumber: 6, endColumn: 2 },
                targetSelectionRange: { startLineNumber: 4, startColumn: 5, endLineNumber: 4, endColumn: 11 },
            },
        ])
    })

    test('응답 후 토큰이 취소돼 있으면 null 을 반환하고 모델을 프리로드하지 않는다', async () => {
        const client = await createTestLspClient({ definitionProvider: true }, () => ({
            uri: 'file:///workspace/target.ts',
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
        }))
        const { monaco, captured, hasModelFor } = createFakeMonaco()
        registerDefinition(monaco, client, 'typescript')

        const result = await captured.definition?.provideDefinition(fakeModel, fakePosition, createFakeToken(true))

        expect(result).toBeNull()
        expect(hasModelFor('/workspace/target.ts')).toBe(false)
    })

    test('결과가 없으면 null 을 반환한다', async () => {
        const client = await createTestLspClient({ definitionProvider: true }, () => null)
        const { monaco, captured } = createFakeMonaco()
        registerDefinition(monaco, client, 'typescript')

        const result = await captured.definition?.provideDefinition(fakeModel, fakePosition, createFakeToken(false))

        expect(result).toBeNull()
    })

    test('대상 파일 모델이 이미 있으면(다른 탭에서 열려있음) 프리로드 없이 그대로 응답한다', async () => {
        const client = await createTestLspClient({ definitionProvider: true }, () => ({
            uri: 'file:///workspace/target.ts',
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
        }))
        const { monaco, captured } = createFakeMonaco()
        monaco.editor.createModel('existing content', 'typescript', monaco.Uri.file('/workspace/target.ts'))
        registerDefinition(monaco, client, 'typescript')

        const result = await captured.definition?.provideDefinition(fakeModel, fakePosition, createFakeToken(false))

        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/target.ts'), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } },
        ])
    })

    test('프리로드용 파일 읽기가 실패해도(예: IPC 불가) definition 응답 자체는 영향받지 않는다', async () => {
        const client = await createTestLspClient({ definitionProvider: true }, () => ({
            uri: 'file:///workspace/target.ts',
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
        }))
        const { monaco, captured, hasModelFor } = createFakeMonaco()
        registerDefinition(monaco, client, 'typescript')

        const result = await captured.definition?.provideDefinition(fakeModel, fakePosition, createFakeToken(false))

        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/target.ts'), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } },
        ])
        expect(hasModelFor('/workspace/target.ts')).toBe(false)
    })
})

describe('createLocationRequestAdapter 로 만들어진 나머지 3종', () => {
    test('registerImplementation 은 implementationProvider 게이트로 registerImplementationProvider 에 등록한다', async () => {
        const unsupported = await createTestLspClient({}, () => null)
        const { monaco: unsupportedMonaco, captured: unsupportedCaptured } = createFakeMonaco()
        registerImplementation(unsupportedMonaco, unsupported, 'typescript')
        expect(unsupportedCaptured.implementation).toBeUndefined()

        const supported = await createTestLspClient({ implementationProvider: true }, () => ({
            uri: 'file:///workspace/impl.ts',
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
        }))
        const { monaco, captured } = createFakeMonaco()
        registerImplementation(monaco, supported, 'typescript')

        const result = await captured.implementation?.provideImplementation(fakeModel, fakePosition, createFakeToken(false))
        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/impl.ts'), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } },
        ])
    })

    test('registerTypeDefinition 은 typeDefinitionProvider 게이트로 registerTypeDefinitionProvider 에 등록한다', async () => {
        const unsupported = await createTestLspClient({}, () => null)
        const { monaco: unsupportedMonaco, captured: unsupportedCaptured } = createFakeMonaco()
        registerTypeDefinition(unsupportedMonaco, unsupported, 'typescript')
        expect(unsupportedCaptured.typeDefinition).toBeUndefined()

        const supported = await createTestLspClient({ typeDefinitionProvider: true }, () => ({
            uri: 'file:///workspace/type.ts',
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
        }))
        const { monaco, captured } = createFakeMonaco()
        registerTypeDefinition(monaco, supported, 'typescript')

        const result = await captured.typeDefinition?.provideTypeDefinition(fakeModel, fakePosition, createFakeToken(false))
        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/type.ts'), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } },
        ])
    })

    test('registerDeclaration 은 declarationProvider 미지원 서버(gopls/tsls)에서 자동으로 무해하다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, captured } = createFakeMonaco()

        const disposable = registerDeclaration(monaco, client, 'go')

        expect(captured.declaration).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('registerDeclaration 은 declarationProvider 게이트로 registerDeclarationProvider 에 등록한다', async () => {
        const client = await createTestLspClient({ declarationProvider: true }, () => ({
            uri: 'file:///workspace/decl.rs',
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
        }))
        const { monaco, captured } = createFakeMonaco()
        registerDeclaration(monaco, client, 'rust')

        const result = await captured.declaration?.provideDeclaration(fakeModel, fakePosition, createFakeToken(false))
        expect(result).toEqual([
            { uri: monaco.Uri.parse('file:///workspace/decl.rs'), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } },
        ])
    })
})
