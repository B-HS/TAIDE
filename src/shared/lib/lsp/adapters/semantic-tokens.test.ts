import { describe, expect, test } from 'bun:test'
import type { CancellationToken, languages } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { SemanticTokensLegend, ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import { toSemanticTokenLegendScope } from '@shared/lib/theme-convert/semantic-token-map'
import {
    applySemanticTokensDeltaEdits,
    buildSemanticTokensLegendMapping,
    decodeSemanticTokensData,
    reencodeSemanticTokens,
    registerSemanticTokens,
    triggerSemanticTokensRefresh,
} from '@shared/lib/lsp/adapters/semantic-tokens'

const createFakeToken = (isCancellationRequested: boolean): CancellationToken => ({
    isCancellationRequested,
    onCancellationRequested: () => ({ dispose: () => {} }),
})

describe('decodeSemanticTokensData', () => {
    test('line/char 상대 인코딩을 절대 좌표로 복원한다', () => {
        const decoded = decodeSemanticTokensData([0, 0, 3, 0, 0, 0, 5, 2, 1, 0, 1, 2, 4, 2, 4])
        expect(decoded).toEqual([
            { line: 0, char: 0, length: 3, typeIndex: 0, modifierBitmask: 0 },
            { line: 0, char: 5, length: 2, typeIndex: 1, modifierBitmask: 0 },
            { line: 1, char: 2, length: 4, typeIndex: 2, modifierBitmask: 4 },
        ])
    })

    test('빈 데이터는 빈 배열을 반환한다', () => {
        expect(decodeSemanticTokensData([])).toEqual([])
    })
})

describe('reencodeSemanticTokens', () => {
    test('미매핑 타입 토큰을 드롭하고 다음 토큰의 delta 에 흡수시킨다', () => {
        const decoded = [
            { line: 0, char: 0, length: 3, typeIndex: 0, modifierBitmask: 1 },
            { line: 0, char: 5, length: 2, typeIndex: 1, modifierBitmask: 0 },
            { line: 1, char: 2, length: 4, typeIndex: 2, modifierBitmask: 4 },
        ]
        const typeIndexByServerIndex = [2, undefined, 1]
        const modifierIndexByServerIndex = [0, undefined, 2]

        const result = reencodeSemanticTokens(decoded, typeIndexByServerIndex, modifierIndexByServerIndex)
        expect(Array.from(result)).toEqual([0, 0, 3, 2, 1, 1, 2, 4, 1, 4])
    })

    test('비표준 modifier 비트는 제거하고 표준 비트만 재매핑한다', () => {
        const decoded = [{ line: 2, char: 0, length: 1, typeIndex: 0, modifierBitmask: 0b010 }]
        const typeIndexByServerIndex = [0]
        const modifierIndexByServerIndex = [undefined]

        const result = reencodeSemanticTokens(decoded, typeIndexByServerIndex, modifierIndexByServerIndex)
        expect(Array.from(result)).toEqual([2, 0, 1, 0, 0])
    })

    test('모든 토큰이 드롭되면 빈 스트림을 반환한다', () => {
        const decoded = [{ line: 0, char: 0, length: 1, typeIndex: 0, modifierBitmask: 0 }]
        const result = reencodeSemanticTokens(decoded, [undefined], [])
        expect(result.length).toBe(0)
    })
})

describe('applySemanticTokensDeltaEdits', () => {
    test('start/deleteCount/data 로 splice 를 적용한다', () => {
        const base = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        const result = applySemanticTokensDeltaEdits(base, [{ start: 5, deleteCount: 5, data: [100, 200, 300, 400, 500] }])
        expect(result).toEqual([1, 2, 3, 4, 5, 100, 200, 300, 400, 500])
    })

    test('data 가 없는 edit 은 순수 삭제로 처리한다', () => {
        const base = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        const result = applySemanticTokensDeltaEdits(base, [{ start: 0, deleteCount: 5 }])
        expect(result).toEqual([6, 7, 8, 9, 10])
    })

    test('여러 edit 의 start 는 원본 배열 인덱스 기준이다 (역순 적용 — monaco documentSemanticTokens.js 참조 구현과 동치)', () => {
        const base = [1, 2, 3, 4, 5, 6]
        const result = applySemanticTokensDeltaEdits(base, [
            { start: 0, deleteCount: 2, data: [9] },
            { start: 4, deleteCount: 1, data: [8, 8] },
        ])
        expect(result).toEqual([9, 3, 4, 8, 8, 6])
    })

    test('원본 배열을 변경하지 않는다', () => {
        const base = [1, 2, 3]
        applySemanticTokensDeltaEdits(base, [{ start: 0, deleteCount: 1, data: [9] }])
        expect(base).toEqual([1, 2, 3])
    })
})

describe('buildSemanticTokensLegendMapping', () => {
    test('매핑되지 않는 타입은 제외하고 SYNTAX_TOKENS 순서로 자체 legend 를 만들며, 각 이름은 taideSemantic. 네임스페이스로 감싼다', () => {
        const serverLegend: SemanticTokensLegend = { tokenTypes: ['namespace', 'unknownType', 'variable', 'keyword'], tokenModifiers: [] }
        const mapping = buildSemanticTokensLegendMapping(serverLegend)
        expect(mapping.legend.tokenTypes).toEqual(['taideSemantic.keyword', 'taideSemantic.variable', 'taideSemantic.namespace'])
        expect(mapping.typeIndexByServerIndex).toEqual([2, undefined, 1, 0])
    })

    test('legend 의 tokenTypes 이름은 어떤 실제 TextMate scope 와도 겹치지 않는다 (워시아웃 방지)', () => {
        const serverLegend: SemanticTokensLegend = { tokenTypes: ['variable', 'keyword', 'string', 'comment'], tokenModifiers: [] }
        const mapping = buildSemanticTokensLegendMapping(serverLegend)
        for (const bareName of ['variable', 'keyword', 'string', 'comment']) {
            expect(mapping.legend.tokenTypes).not.toContain(bareName)
        }
        expect(mapping.legend.tokenTypes.every((type) => type.startsWith('taideSemantic.'))).toBe(true)
    })

    test('modifier 는 표준 10종을 그대로 노출하고, 서버 인덱스를 표준 인덱스로 재매핑한다', () => {
        const serverLegend: SemanticTokensLegend = { tokenTypes: [], tokenModifiers: ['declaration', 'unknownMod', 'readonly'] }
        const mapping = buildSemanticTokensLegendMapping(serverLegend)
        expect(mapping.legend.tokenModifiers).toEqual([
            'declaration',
            'definition',
            'readonly',
            'static',
            'deprecated',
            'abstract',
            'async',
            'modification',
            'documentation',
            'defaultLibrary',
        ])
        expect(mapping.modifierIndexByServerIndex).toEqual([0, undefined, 2])
    })

    test('rust-analyzer 비표준 타입 별칭(builtinType/lifetime/macroBang)을 SYNTAX_TOKENS 로 매핑한다', () => {
        const serverLegend: SemanticTokensLegend = { tokenTypes: ['builtinType', 'lifetime', 'macroBang'], tokenModifiers: [] }
        const mapping = buildSemanticTokensLegendMapping(serverLegend)
        expect([...mapping.legend.tokenTypes].sort()).toEqual(
            [toSemanticTokenLegendScope('function'), toSemanticTokenLegendScope('storage'), toSemanticTokenLegendScope('type')].sort(),
        )
    })

    test('Object.prototype 멤버와 이름이 겹치는 서버 타입은 매핑되지 않고 드롭된다 (프로토타입 체인 오염 방지)', () => {
        const serverLegend: SemanticTokensLegend = { tokenTypes: ['constructor', 'toString', 'hasOwnProperty', 'variable'], tokenModifiers: [] }
        const mapping = buildSemanticTokensLegendMapping(serverLegend)
        expect(mapping.legend.tokenTypes).toEqual([toSemanticTokenLegendScope('variable')])
        expect(mapping.typeIndexByServerIndex).toEqual([undefined, undefined, undefined, 0])
    })
})

type SemanticTokensProviderArg = Parameters<Monaco['languages']['registerDocumentSemanticTokensProvider']>[1]

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
    let capturedProvider: SemanticTokensProviderArg | undefined
    let disposeCallCount = 0
    const fakeMonaco = {
        Emitter: FakeEmitter,
        languages: {
            registerDocumentSemanticTokensProvider: (_languageId: string, provider: SemanticTokensProviderArg) => {
                capturedProvider = provider
                return { dispose: () => (disposeCallCount += 1) }
            },
        },
    }
    return { monaco: fakeMonaco as unknown as Monaco, getProvider: () => capturedProvider, getDisposeCallCount: () => disposeCallCount }
}

const fakeModelAt = (uri: string) =>
    ({ uri: { toString: () => uri } }) as Parameters<languages.DocumentSemanticTokensProvider['provideDocumentSemanticTokens']>[0]

/** This adapter never answers with `SemanticTokensEdits` (only `SemanticTokens`) — narrows the union so tests can read `.data` directly. */
const dataOf = (result: languages.SemanticTokens | languages.SemanticTokensEdits | null | undefined) => {
    if (!result || !('data' in result)) throw new Error('expected a full SemanticTokens result, got SemanticTokensEdits')
    return Array.from(result.data)
}

const SINGLE_VARIABLE_CAPABILITIES: ServerCapabilities = {
    semanticTokensProvider: { legend: { tokenTypes: ['variable'], tokenModifiers: [] }, full: { delta: true } },
}

describe('registerSemanticTokens', () => {
    test('서버가 semanticTokensProvider.full 을 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        const { monaco, getProvider } = createFakeMonaco()
        const disposable = registerSemanticTokens(monaco, client, 'typescript')
        expect(getProvider()).toBeUndefined()
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('isEnabled 가 false 면 요청 없이 빈 토큰을 반환한다', async () => {
        let requestCount = 0
        const client = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, () => {
            requestCount += 1
            return { resultId: 'r1', data: [0, 0, 1, 0, 0] }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript', () => false)

        const result = await getProvider()?.provideDocumentSemanticTokens(fakeModelAt('file:///a.ts'), null, createFakeToken(false))
        expect(result).toEqual({ resultId: undefined, data: new Uint32Array(0) })
        expect(requestCount).toBe(0)
    })

    test('최초 호출은 full 요청 결과를 매핑된 legend 인덱스로 재인코딩해 반환하고 캐시한다', async () => {
        const capabilities: ServerCapabilities = {
            semanticTokensProvider: {
                legend: {
                    tokenTypes: ['keyword', 'unknownType', 'variable', 'namespace'],
                    tokenModifiers: ['declaration', 'unknownMod', 'readonly'],
                },
                full: { delta: true },
            },
        }
        const client = await createTestLspClient(capabilities, (method) => {
            if (method !== 'textDocument/semanticTokens/full') throw new Error(`unexpected method: ${method}`)
            return { resultId: 'r1', data: [0, 0, 3, 0, 1, 0, 5, 2, 1, 0, 1, 2, 4, 2, 4] }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript')

        const result = await getProvider()?.provideDocumentSemanticTokens(fakeModelAt('file:///a.ts'), null, createFakeToken(false))
        expect(result?.resultId).toBe('r1')
        expect(dataOf(result)).toEqual([0, 0, 3, 0, 1, 1, 2, 4, 1, 4])
    })

    test('직전 resultId 가 캐시와 일치하면 delta 요청을 보내고, 캐시된 서버 데이터에 splice 적용 후 재인코딩한다', async () => {
        let fullRequestCount = 0
        let deltaRequestCount = 0
        let lastDeltaParams: unknown
        const client = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, (method, params) => {
            if (method === 'textDocument/semanticTokens/full') {
                fullRequestCount += 1
                return { resultId: 'r1', data: [0, 0, 3, 0, 0] }
            }
            deltaRequestCount += 1
            lastDeltaParams = params
            return { resultId: 'r2', edits: [{ start: 0, deleteCount: 5, data: [0, 0, 3, 0, 0, 1, 0, 2, 0, 0] }] }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript')
        const model = fakeModelAt('file:///a.ts')

        const first = await getProvider()?.provideDocumentSemanticTokens(model, null, createFakeToken(false))
        expect(first?.resultId).toBe('r1')

        const second = await getProvider()?.provideDocumentSemanticTokens(model, 'r1', createFakeToken(false))
        expect(fullRequestCount).toBe(1)
        expect(deltaRequestCount).toBe(1)
        expect(lastDeltaParams).toEqual({ textDocument: { uri: 'file:///a.ts' }, previousResultId: 'r1' })
        expect(second?.resultId).toBe('r2')
        expect(dataOf(second)).toEqual([0, 0, 3, 0, 0, 1, 0, 2, 0, 0])
    })

    test('lastResultId 가 캐시와 다르면 delta 를 시도하지 않고 full 을 재요청한다', async () => {
        let fullRequestCount = 0
        let deltaRequestCount = 0
        const client = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, (method) => {
            if (method === 'textDocument/semanticTokens/full') {
                fullRequestCount += 1
                return { resultId: 'r1', data: [0, 0, 1, 0, 0] }
            }
            deltaRequestCount += 1
            return { resultId: 'r2', edits: [] }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript')
        const model = fakeModelAt('file:///a.ts')

        await getProvider()?.provideDocumentSemanticTokens(model, null, createFakeToken(false))
        await getProvider()?.provideDocumentSemanticTokens(model, 'some-other-id', createFakeToken(false))

        expect(fullRequestCount).toBe(2)
        expect(deltaRequestCount).toBe(0)
    })

    test('서버가 delta 를 광고하지 않으면 캐시가 있어도 항상 full 을 요청한다', async () => {
        let fullRequestCount = 0
        let deltaRequestCount = 0
        const capabilities: ServerCapabilities = {
            semanticTokensProvider: { legend: { tokenTypes: ['variable'], tokenModifiers: [] }, full: true },
        }
        const client = await createTestLspClient(capabilities, (method) => {
            if (method === 'textDocument/semanticTokens/full') {
                fullRequestCount += 1
                return { resultId: 'r1', data: [0, 0, 1, 0, 0] }
            }
            deltaRequestCount += 1
            return null
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript')
        const model = fakeModelAt('file:///a.ts')

        await getProvider()?.provideDocumentSemanticTokens(model, null, createFakeToken(false))
        await getProvider()?.provideDocumentSemanticTokens(model, 'r1', createFakeToken(false))

        expect(fullRequestCount).toBe(2)
        expect(deltaRequestCount).toBe(0)
    })

    test('delta 요청에 서버가 edits 대신 SemanticTokens(full) 로 응답해도 그대로 사용한다', async () => {
        const client = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, (method) => {
            if (method === 'textDocument/semanticTokens/full') return { resultId: 'r1', data: [0, 0, 1, 0, 0] }
            return { resultId: 'r3', data: [0, 0, 2, 0, 0] }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript')
        const model = fakeModelAt('file:///a.ts')

        await getProvider()?.provideDocumentSemanticTokens(model, null, createFakeToken(false))
        const second = await getProvider()?.provideDocumentSemanticTokens(model, 'r1', createFakeToken(false))

        expect(second?.resultId).toBe('r3')
        expect(dataOf(second)).toEqual([0, 0, 2, 0, 0])
    })

    test('요청 완료 후 토큰이 취소돼 있으면 결과를 폐기하고 캐시도 갱신하지 않는다', async () => {
        let fullRequestCount = 0
        const client = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, () => {
            fullRequestCount += 1
            return { resultId: 'r1', data: [0, 0, 1, 0, 0] }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript')
        const model = fakeModelAt('file:///a.ts')

        const cancelled = await getProvider()?.provideDocumentSemanticTokens(model, null, createFakeToken(true))
        expect(cancelled).toEqual({ resultId: undefined, data: new Uint32Array(0) })

        await getProvider()?.provideDocumentSemanticTokens(model, 'r1', createFakeToken(false))
        expect(fullRequestCount).toBe(2)
    })

    test('releaseDocumentSemanticTokens 는 해당 resultId 를 가진 모델의 캐시를 해제한다', async () => {
        let fullRequestCount = 0
        let deltaRequestCount = 0
        const client = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, (method) => {
            if (method === 'textDocument/semanticTokens/full') {
                fullRequestCount += 1
                return { resultId: 'r1', data: [0, 0, 1, 0, 0] }
            }
            deltaRequestCount += 1
            return { resultId: 'r2', edits: [] }
        })
        const { monaco, getProvider } = createFakeMonaco()
        registerSemanticTokens(monaco, client, 'typescript')
        const model = fakeModelAt('file:///a.ts')

        await getProvider()?.provideDocumentSemanticTokens(model, null, createFakeToken(false))
        getProvider()?.releaseDocumentSemanticTokens('r1')
        await getProvider()?.provideDocumentSemanticTokens(model, 'r1', createFakeToken(false))

        expect(fullRequestCount).toBe(2)
        expect(deltaRequestCount).toBe(0)
    })

    test('dispose 는 provider 등록을 해제하고 이후 refresh 트리거에 반응하지 않는다', async () => {
        const client = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, () => ({ resultId: 'r1', data: [] }))
        const { monaco, getProvider, getDisposeCallCount } = createFakeMonaco()
        const disposable = registerSemanticTokens(monaco, client, 'typescript')

        let fired = false
        getProvider()?.onDidChange?.(() => (fired = true))
        disposable.dispose()
        expect(getDisposeCallCount()).toBe(1)

        triggerSemanticTokensRefresh(client)
        expect(fired).toBe(false)
    })

    test('triggerSemanticTokensRefresh 는 같은 client 로 등록된 registration 의 onDidChange 만 발화한다', async () => {
        const clientA = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, () => ({ resultId: 'r1', data: [] }))
        const clientB = await createTestLspClient(SINGLE_VARIABLE_CAPABILITIES, () => ({ resultId: 'r1', data: [] }))
        const { monaco: monacoA, getProvider: getProviderA } = createFakeMonaco()
        const { monaco: monacoB, getProvider: getProviderB } = createFakeMonaco()
        registerSemanticTokens(monacoA, clientA, 'typescript')
        registerSemanticTokens(monacoB, clientB, 'go')

        let firedA = false
        let firedB = false
        getProviderA()?.onDidChange?.(() => (firedA = true))
        getProviderB()?.onDidChange?.(() => (firedB = true))

        triggerSemanticTokensRefresh(clientA)

        expect(firedA).toBe(true)
        expect(firedB).toBe(false)
    })
})
