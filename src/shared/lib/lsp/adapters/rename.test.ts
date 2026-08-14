import { describe, expect, test } from 'bun:test'
import type { languages } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import { registerRename } from '@shared/lib/lsp/adapters/rename'

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

type RenameProviderArg = Parameters<Monaco['languages']['registerRenameProvider']>[1]

const createFakeMonaco = (model: unknown = null) => {
    let capturedProvider: RenameProviderArg | undefined
    const fakeMonaco = {
        Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
        Range: class {
            constructor(
                public startLineNumber: number,
                public startColumn: number,
                public endLineNumber: number,
                public endColumn: number,
            ) {}
            static fromPositions(start: { lineNumber: number; column: number }, end: { lineNumber: number; column: number }) {
                return { startLineNumber: start.lineNumber, startColumn: start.column, endLineNumber: end.lineNumber, endColumn: end.column }
            }
        },
        editor: { getModel: () => model, getEditors: () => [] as { getModel: () => unknown }[] },
        languages: {
            registerRenameProvider: (_languageId: string, provider: RenameProviderArg) => {
                capturedProvider = provider
                return { dispose: () => {} }
            },
        },
    }
    return { monaco: fakeMonaco as unknown as Monaco, getProvider: () => capturedProvider }
}

const fakeModel = { uri: { toString: () => 'file:///a.ts' }, getWordAtPosition: () => null } as unknown as Parameters<
    languages.RenameProvider['provideRenameEdits']
>[0]
const fakePosition = { lineNumber: 1, column: 1 } as Parameters<languages.RenameProvider['provideRenameEdits']>[1]
const fakeToken = { isCancellationRequested: false, onCancellationRequested: () => ({ dispose: () => {} }) } as Parameters<
    languages.RenameProvider['provideRenameEdits']
>[3]

describe('registerRename', () => {
    test('서버가 renameProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        registerRename(monaco, client, 'typescript')
        expect(getProvider()).toBeUndefined()
    })

    test('rename 결과가 없으면 빈 edits 를 반환한다', async () => {
        const client = await createTestLspClient({ renameProvider: true }, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        registerRename(monaco, client, 'typescript')

        const result = await getProvider()?.provideRenameEdits(fakeModel, fakePosition, 'newName', fakeToken)
        expect(result).toEqual({ edits: [] })
    })

    test('열린 모델에 적용을 성공하면 빈 edits(모나코 자체 적용은 no-op)를 반환한다', async () => {
        const pushed: unknown[] = []
        const fakeTargetModel = { pushEditOperations: (_before: unknown, ops: unknown) => pushed.push(ops) }
        const client = await createTestLspClient({ renameProvider: true }, () => ({
            changes: { 'file:///a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 3 } }, newText: 'renamed' }] },
        }))
        const { monaco, getProvider } = createFakeMonaco(fakeTargetModel)
        registerRename(monaco, client, 'typescript')

        const result = await getProvider()?.provideRenameEdits(fakeModel, fakePosition, 'renamed', fakeToken)

        expect(result).toEqual({ edits: [] })
        expect(pushed).toHaveLength(1)
    })

    test('적용이 실패하면 rejectReason 을 담아 반환한다 (edits 는 여전히 빈 배열)', async () => {
        const throwingModel = {
            pushEditOperations: () => {
                throw new Error('apply failed')
            },
        }
        const client = await createTestLspClient({ renameProvider: true }, () => ({
            changes: { 'file:///a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 3 } }, newText: 'renamed' }] },
        }))
        const { monaco, getProvider } = createFakeMonaco(throwingModel)
        registerRename(monaco, client, 'typescript')

        const result = await getProvider()?.provideRenameEdits(fakeModel, fakePosition, 'renamed', fakeToken)

        expect(result?.edits).toEqual([])
        expect(result?.rejectReason).toBe('apply failed')
    })

    test('prepareProvider 를 지원하지 않으면 resolveRenameLocation 을 노출하지 않는다', async () => {
        const client = await createTestLspClient({ renameProvider: true }, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        registerRename(monaco, client, 'typescript')
        expect(getProvider()?.resolveRenameLocation).toBeUndefined()
    })

    test('prepareRename 결과가 없으면 rejectReason 과 함께 커서 단어로 폴백한다', async () => {
        const client = await createTestLspClient({ renameProvider: { prepareProvider: true } }, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        registerRename(monaco, client, 'typescript')

        const wordModel = {
            ...fakeModel,
            getWordAtPosition: () => ({ word: 'foo', startColumn: 1, endColumn: 4 }),
        } as Parameters<languages.RenameProvider['provideRenameEdits']>[0]
        const result = await getProvider()?.resolveRenameLocation?.(wordModel, fakePosition, {} as never)

        expect(result?.text).toBe('foo')
        expect(result?.rejectReason).toBeTruthy()
    })
})
