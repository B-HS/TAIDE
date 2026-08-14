import { describe, expect, test } from 'bun:test'
import type { CancellationToken, languages } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import { FOLDING_RANGE_CLIENT_LIMIT, registerFoldingRange, toMonacoFoldingRangeLines } from '@shared/lib/lsp/adapters/folding-range'

const createFakeToken = (isCancellationRequested: boolean): CancellationToken => ({
    isCancellationRequested,
    onCancellationRequested: () => ({ dispose: () => {} }),
})

describe('toMonacoFoldingRangeLines', () => {
    test('0-based LSP 라인을 1-based monaco 라인으로 변환한다', () => {
        expect(toMonacoFoldingRangeLines({ startLine: 0, endLine: 4 })).toEqual({ start: 1, end: 5 })
    })

    test('시작과 끝이 같은 단일 라인도 정확히 오프셋한다', () => {
        expect(toMonacoFoldingRangeLines({ startLine: 9, endLine: 9 })).toEqual({ start: 10, end: 10 })
    })
})

type FoldingRangeProviderArg = Parameters<Monaco['languages']['registerFoldingRangeProvider']>[1]

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

const createFakeMonaco = () => {
    let capturedProvider: FoldingRangeProviderArg | undefined
    let disposeCallCount = 0
    const fakeMonaco = {
        languages: {
            FoldingRangeKind: { fromValue: (value: string) => ({ value }) },
            registerFoldingRangeProvider: (_languageId: string, provider: FoldingRangeProviderArg) => {
                capturedProvider = provider
                return { dispose: () => (disposeCallCount += 1) }
            },
        },
    }
    return { monaco: fakeMonaco as unknown as Monaco, getProvider: () => capturedProvider, getDisposeCallCount: () => disposeCallCount }
}

const fakeModel = { uri: { toString: () => 'file:///a.ts' } } as Parameters<languages.FoldingRangeProvider['provideFoldingRanges']>[0]
const fakeContext = {} as Parameters<languages.FoldingRangeProvider['provideFoldingRanges']>[1]

describe('registerFoldingRange', () => {
    test('서버가 foldingRangeProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        const disposable = registerFoldingRange(monaco, client, 'typescript')
        expect(getProvider()).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('LSP foldingRange 응답을 1-based monaco FoldingRange 로 변환한다', async () => {
        const client = await createTestLspClient({ foldingRangeProvider: true }, () => [
            { startLine: 0, endLine: 3, kind: 'imports' },
            { startLine: 5, endLine: 10 },
        ])
        const { monaco, getProvider } = createFakeMonaco()
        registerFoldingRange(monaco, client, 'typescript')

        const result = await getProvider()?.provideFoldingRanges(fakeModel, fakeContext, createFakeToken(false))
        expect(result).toEqual([
            { start: 1, end: 4, kind: { value: 'imports' } },
            { start: 6, end: 11, kind: undefined },
        ])
    })

    test('rangeLimit 을 초과하는 응답은 클라이언트 측에서 잘라낸다', async () => {
        const oversized = Array.from({ length: FOLDING_RANGE_CLIENT_LIMIT + 10 }, (_, index) => ({ startLine: index, endLine: index }))
        const client = await createTestLspClient({ foldingRangeProvider: true }, () => oversized)
        const { monaco, getProvider } = createFakeMonaco()
        registerFoldingRange(monaco, client, 'typescript')

        const result = await getProvider()?.provideFoldingRanges(fakeModel, fakeContext, createFakeToken(false))
        expect(result).toHaveLength(FOLDING_RANGE_CLIENT_LIMIT)
    })

    test('취소된 토큰이면 서버 응답을 폐기하고 빈 배열을 반환한다', async () => {
        const client = await createTestLspClient({ foldingRangeProvider: true }, () => [{ startLine: 0, endLine: 1 }])
        const { monaco, getProvider } = createFakeMonaco()
        registerFoldingRange(monaco, client, 'typescript')

        const result = await getProvider()?.provideFoldingRanges(fakeModel, fakeContext, createFakeToken(true))
        expect(result).toEqual([])
    })
})
