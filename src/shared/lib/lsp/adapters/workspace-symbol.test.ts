import { describe, expect, test } from 'bun:test'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { ServerCapabilities, SymbolInformation, WorkspaceSymbol } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import {
    WORKSPACE_SYMBOL_SEARCH_DEBOUNCE_MS,
    createWorkspaceSymbolSearch,
    mergeWorkspaceSymbolResults,
    requestWorkspaceSymbols,
} from '@shared/lib/lsp/adapters/workspace-symbol'

const createFakeMonaco = (): Monaco =>
    ({
        Uri: {
            parse: (uri: string) => {
                const scheme = /^([a-zA-Z][a-zA-Z0-9+.-]*):/.exec(uri)?.[1] ?? ''
                return { scheme, fsPath: uri.replace(`${scheme}://`, '') }
            },
        },
    }) as unknown as Monaco

const createTestLspClient = async (
    capabilities: ServerCapabilities,
    handleRequest: (method: string, params: unknown) => unknown,
    onRequestSent?: (method: string, params: unknown) => void,
) => {
    const client = createLspClient({
        send: (message) => {
            if (!isJsonRpcRequest(message)) return
            if (message.method === 'initialize') {
                client.handleMessage({ jsonrpc: '2.0', id: message.id, result: { capabilities } })
                return
            }
            onRequestSent?.(message.method, message.params)
            client.handleMessage({ jsonrpc: '2.0', id: message.id, result: handleRequest(message.method, message.params) })
        },
        onNotification: () => {},
    })
    await client.initialize({})
    return client
}

const symbolInformation = (name: string, path: string, line: number, character: number): SymbolInformation => ({
    name,
    kind: 12,
    location: { uri: `file://${path}`, range: { start: { line, character }, end: { line, character: character + name.length } } },
})

describe('mergeWorkspaceSymbolResults', () => {
    test('여러 세션의 성공 응답을 하나의 배열로 합친다', () => {
        const monaco = createFakeMonaco()
        const settled: PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[] = [
            { status: 'fulfilled', value: [symbolInformation('foo', '/a.ts', 0, 0)] },
            { status: 'fulfilled', value: [symbolInformation('bar', '/b.ts', 1, 2)] },
        ]

        const result = mergeWorkspaceSymbolResults(monaco, settled)

        expect(result.map((symbol) => symbol.name)).toEqual(['foo', 'bar'])
    })

    test('rejected 세션(미지원 서버 등)은 결과에서 조용히 제외된다', () => {
        const monaco = createFakeMonaco()
        const settled: PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[] = [
            { status: 'fulfilled', value: [symbolInformation('foo', '/a.ts', 0, 0)] },
            { status: 'rejected', reason: new Error('capability not supported') },
        ]

        const result = mergeWorkspaceSymbolResults(monaco, settled)

        expect(result.map((symbol) => symbol.name)).toEqual(['foo'])
    })

    test('null 응답은 빈 결과로 취급한다', () => {
        const monaco = createFakeMonaco()
        const settled: PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[] = [{ status: 'fulfilled', value: null }]

        expect(mergeWorkspaceSymbolResults(monaco, settled)).toEqual([])
    })

    test('location 이 lazy({uri} 만)인 항목은 아직 위치를 알 수 없으므로 제외한다(resolve 미구현)', () => {
        const monaco = createFakeMonaco()
        const lazy: WorkspaceSymbol = { name: 'lazy', kind: 12, location: { uri: 'file:///lazy.ts' } }
        const settled: PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[] = [{ status: 'fulfilled', value: [lazy] }]

        expect(mergeWorkspaceSymbolResults(monaco, settled)).toEqual([])
    })

    test('LSP 0-based 위치를 monaco 1-based line/column 으로, uri 를 path 로 정규화한다', () => {
        const monaco = createFakeMonaco()
        const settled: PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[] = [
            { status: 'fulfilled', value: [symbolInformation('handleSave', '/workspace/src/foo.ts', 4, 10)] },
        ]

        expect(mergeWorkspaceSymbolResults(monaco, settled)).toEqual([
            { name: 'handleSave', kind: 12, containerName: undefined, path: '/workspace/src/foo.ts', line: 5, column: 11 },
        ])
    })

    test('file 스킴이 아닌 위치(jdt:// 등 가상 문서)는 조용히 제외한다', () => {
        const monaco = createFakeMonaco()
        const virtualDoc: SymbolInformation = {
            name: 'VirtualClass',
            kind: 5,
            location: {
                uri: 'jdt://contents/foo.jar/pkg/VirtualClass.class',
                range: { start: { line: 0, character: 0 }, end: { line: 0, character: 5 } },
            },
        }
        const settled: PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[] = [
            { status: 'fulfilled', value: [virtualDoc, symbolInformation('real', '/a.ts', 0, 0)] },
        ]

        expect(mergeWorkspaceSymbolResults(monaco, settled).map((symbol) => symbol.name)).toEqual(['real'])
    })

    test('containerName 이 있으면 보존한다(WorkspaceSymbol)', () => {
        const monaco = createFakeMonaco()
        const withContainer: WorkspaceSymbol = {
            name: 'method',
            kind: 6,
            containerName: 'MyClass',
            location: { uri: 'file:///a.ts', range: { start: { line: 0, character: 0 }, end: { line: 0, character: 6 } } },
        }
        const settled: PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[] = [{ status: 'fulfilled', value: [withContainer] }]

        expect(mergeWorkspaceSymbolResults(monaco, settled)[0]?.containerName).toBe('MyClass')
    })
})

describe('requestWorkspaceSymbols', () => {
    test('모든 클라이언트에 workspace/symbol 요청을 보내고 결과를 합친다', async () => {
        const monaco = createFakeMonaco()
        const sentQueries: unknown[] = []
        const clientA = await createTestLspClient(
            { workspaceSymbolProvider: true },
            () => [symbolInformation('foo', '/a.ts', 0, 0)],
            (_method, params) => sentQueries.push(params),
        )
        const clientB = await createTestLspClient(
            { workspaceSymbolProvider: true },
            () => [symbolInformation('bar', '/b.ts', 0, 0)],
            (_method, params) => sentQueries.push(params),
        )

        const result = await requestWorkspaceSymbols(monaco, [clientA, clientB], 'foo')

        expect(sentQueries).toEqual([{ query: 'foo' }, { query: 'foo' }])
        expect(result.map((symbol) => symbol.name).toSorted()).toEqual(['bar', 'foo'])
    })

    test('workspaceSymbolProvider 를 광고하지 않는 세션은 조용히 제외된다(다른 세션은 정상 반환)', async () => {
        const monaco = createFakeMonaco()
        const unsupported = await createTestLspClient({}, () => [symbolInformation('foo', '/a.ts', 0, 0)])
        const supported = await createTestLspClient({ workspaceSymbolProvider: true }, () => [symbolInformation('bar', '/b.ts', 0, 0)])

        const result = await requestWorkspaceSymbols(monaco, [unsupported, supported], 'x')

        expect(result.map((symbol) => symbol.name)).toEqual(['bar'])
    })

    test('클라이언트가 없으면 빈 배열을 반환한다', async () => {
        const monaco = createFakeMonaco()
        expect(await requestWorkspaceSymbols(monaco, [], 'x')).toEqual([])
    })
})

describe('createWorkspaceSymbolSearch', () => {
    test('디바운스 지연 전에는 요청을 보내지 않는다', async () => {
        const monaco = createFakeMonaco()
        let requestCount = 0
        const client = await createTestLspClient({ workspaceSymbolProvider: true }, () => {
            requestCount += 1
            return []
        })
        const controller = createWorkspaceSymbolSearch(monaco)

        void controller.search([client], 'foo')
        expect(requestCount).toBe(0)
    })

    test('디바운스 지연 후 정확히 한 번 요청해 결과를 반환한다', async () => {
        const monaco = createFakeMonaco()
        let requestCount = 0
        const client = await createTestLspClient({ workspaceSymbolProvider: true }, () => {
            requestCount += 1
            return [symbolInformation('foo', '/a.ts', 0, 0)]
        })
        const controller = createWorkspaceSymbolSearch(monaco)

        const result = await controller.search([client], 'foo')

        expect(requestCount).toBe(1)
        expect(result.map((symbol) => symbol.name)).toEqual(['foo'])
    })

    test('디바운스 창 안에서 연속 호출하면 마지막 쿼리로만 한 번 요청한다', async () => {
        const monaco = createFakeMonaco()
        const receivedQueries: unknown[] = []
        const client = await createTestLspClient(
            { workspaceSymbolProvider: true },
            () => [],
            (_method, params) => receivedQueries.push(params),
        )
        const controller = createWorkspaceSymbolSearch(monaco)

        void controller.search([client], 'f')
        void controller.search([client], 'fo')
        const finalResult = await controller.search([client], 'foo')

        expect(finalResult).toEqual([])
        expect(receivedQueries).toEqual([{ query: 'foo' }])
    })

    test('cancel 은 아직 발화하지 않은 타이머를 멈춰 요청 자체를 막는다', async () => {
        const monaco = createFakeMonaco()
        let requestCount = 0
        const client = await createTestLspClient({ workspaceSymbolProvider: true }, () => {
            requestCount += 1
            return []
        })
        const controller = createWorkspaceSymbolSearch(monaco)

        void controller.search([client], 'foo')
        controller.cancel()

        await new Promise((resolve) => setTimeout(resolve, WORKSPACE_SYMBOL_SEARCH_DEBOUNCE_MS + 50))
        expect(requestCount).toBe(0)
    })
})
