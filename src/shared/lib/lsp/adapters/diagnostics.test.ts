import { describe, expect, test } from 'bun:test'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { diagnosticsOwnerFor, getStoredDiagnostics, registerDiagnostics } from '@shared/lib/lsp/adapters/diagnostics'

describe('diagnosticsOwnerFor', () => {
    test('서버 id 별로 서로 다른 marker owner 를 만든다', () => {
        expect(diagnosticsOwnerFor('basedPyright')).toBe('lsp-basedPyright')
        expect(diagnosticsOwnerFor('ruff')).toBe('lsp-ruff')
    })

    test('같은 서버 id 는 같은 owner 를 반환한다', () => {
        expect(diagnosticsOwnerFor('vtsls')).toBe(diagnosticsOwnerFor('vtsls'))
    })

    test('서로 다른 서버의 owner 는 겹치지 않는다', () => {
        expect(diagnosticsOwnerFor('basedPyright')).not.toBe(diagnosticsOwnerFor('ruff'))
    })
})

type FakeModel = { uri: { toString: () => string } }

const createFakeMonaco = () => {
    const markerCalls: { model: FakeModel; owner: string; markers: unknown }[] = []
    const models = new Map<string, FakeModel>()
    let disposeModelListener: ((model: FakeModel) => void) | undefined

    const fakeMonaco = {
        Uri: { parse: (uri: string) => ({ toString: () => uri }) },
        MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 },
        editor: {
            getModel: (uri: { toString: () => string }) => models.get(uri.toString()),
            setModelMarkers: (model: FakeModel, owner: string, markers: unknown) => markerCalls.push({ model, owner, markers }),
            onWillDisposeModel: (listener: (model: FakeModel) => void) => {
                disposeModelListener = listener
                return { dispose: () => (disposeModelListener = undefined) }
            },
        },
    }

    return {
        monaco: fakeMonaco as unknown as Monaco,
        registerModel: (uri: string) => models.set(uri, { uri: { toString: () => uri } }),
        fireModelDisposed: (uri: string) => disposeModelListener?.({ uri: { toString: () => uri } }),
        getMarkerCalls: () => markerCalls,
    }
}

const createFakeClient = () => createLspClient({ send: () => {}, onNotification: () => {} })

const publish = (client: ReturnType<typeof createFakeClient>, params: unknown) =>
    client.handleMessage({ jsonrpc: '2.0', method: 'textDocument/publishDiagnostics', params })

describe('registerDiagnostics 원본 사이드 맵', () => {
    test('알려지지 않은 uri/서버는 빈 배열을 반환한다', () => {
        expect(getStoredDiagnostics('ruff', 'file:///unknown.py')).toEqual([])
    })

    test('publishDiagnostics 원본(code·data·source)을 marker 변환과 별개로 보관한다', () => {
        const { monaco } = createFakeMonaco()
        const client = createFakeClient()
        registerDiagnostics(monaco, client, 'ruff')

        const diagnostics = [
            {
                range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
                message: 'unused import',
                code: 'F401',
                source: 'ruff',
                data: { fix: 'remove' },
            },
        ]
        publish(client, { uri: 'file:///a.py', diagnostics })

        expect(getStoredDiagnostics('ruff', 'file:///a.py')).toEqual(diagnostics)
    })

    test('서로 다른 서버가 같은 uri 를 보고해도 서로의 원본을 덮어쓰지 않는다', () => {
        const { monaco } = createFakeMonaco()
        const ruffClient = createFakeClient()
        const pyrightClient = createFakeClient()
        registerDiagnostics(monaco, ruffClient, 'ruff')
        registerDiagnostics(monaco, pyrightClient, 'basedPyright')

        const ruffDiagnostics = [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, message: 'ruff issue', code: 'F401' }]
        const pyrightDiagnostics = [{ range: { start: { line: 1, character: 0 }, end: { line: 1, character: 1 } }, message: 'type issue' }]
        publish(ruffClient, { uri: 'file:///a.py', diagnostics: ruffDiagnostics })
        publish(pyrightClient, { uri: 'file:///a.py', diagnostics: pyrightDiagnostics })

        expect(getStoredDiagnostics('ruff', 'file:///a.py')).toEqual(ruffDiagnostics)
        expect(getStoredDiagnostics('basedPyright', 'file:///a.py')).toEqual(pyrightDiagnostics)
    })

    test('모델이 열려 있으면 기존과 동일하게 marker 로도 변환한다', () => {
        const { monaco, registerModel, getMarkerCalls } = createFakeMonaco()
        registerModel('file:///a.py')
        const client = createFakeClient()
        registerDiagnostics(monaco, client, 'ruff')

        publish(client, {
            uri: 'file:///a.py',
            diagnostics: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, message: 'issue' }],
        })

        expect(getMarkerCalls()).toHaveLength(1)
        expect(getMarkerCalls()[0]?.owner).toBe('lsp-ruff')
    })

    test('dispose 시 해당 서버가 보관한 원본을 전부 정리한다', () => {
        const { monaco } = createFakeMonaco()
        const client = createFakeClient()
        const disposable = registerDiagnostics(monaco, client, 'ruff')

        publish(client, {
            uri: 'file:///a.py',
            diagnostics: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, message: 'issue' }],
        })
        expect(getStoredDiagnostics('ruff', 'file:///a.py')).toHaveLength(1)

        disposable.dispose()
        expect(getStoredDiagnostics('ruff', 'file:///a.py')).toEqual([])
    })

    test('모델이 닫히면(dispose) 그 uri 의 보관된 원본을 정리한다', () => {
        const { monaco, fireModelDisposed } = createFakeMonaco()
        const client = createFakeClient()
        registerDiagnostics(monaco, client, 'ruff')

        publish(client, {
            uri: 'file:///b.py',
            diagnostics: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, message: 'issue' }],
        })
        expect(getStoredDiagnostics('ruff', 'file:///b.py')).toHaveLength(1)

        fireModelDisposed('file:///b.py')
        expect(getStoredDiagnostics('ruff', 'file:///b.py')).toEqual([])
    })

    test('같은 서버 id 라도 다른 registerDiagnostics 인스턴스(다른 프로젝트 세션)의 원본은 dispose 시 건드리지 않는다', () => {
        const { monaco } = createFakeMonaco()
        const clientA = createFakeClient()
        const clientB = createFakeClient()
        const disposableA = registerDiagnostics(monaco, clientA, 'ruff')
        registerDiagnostics(monaco, clientB, 'ruff')

        publish(clientA, {
            uri: 'file:///project-a/a.py',
            diagnostics: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, message: 'a' }],
        })
        publish(clientB, {
            uri: 'file:///project-b/b.py',
            diagnostics: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, message: 'b' }],
        })

        disposableA.dispose()

        expect(getStoredDiagnostics('ruff', 'file:///project-a/a.py')).toEqual([])
        expect(getStoredDiagnostics('ruff', 'file:///project-b/b.py')).toHaveLength(1)
    })

    test('저장 키는 서버 원본 uri 문자열이 아니라 monaco 정규화 uri 를 사용한다 (인코딩이 달라도 조회된다)', () => {
        const models = new Map<string, FakeModel>()
        const monaco = {
            Uri: { parse: (uri: string) => ({ toString: () => uri.replace('(1)', '%281%29') }) },
            MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 },
            editor: {
                getModel: (uri: { toString: () => string }) => models.get(uri.toString()),
                setModelMarkers: () => {},
                onWillDisposeModel: () => ({ dispose: () => {} }),
            },
        } as unknown as Monaco
        const client = createFakeClient()
        registerDiagnostics(monaco, client, 'ruff')

        publish(client, {
            uri: 'file:///a(1).py',
            diagnostics: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, message: 'issue' }],
        })

        expect(getStoredDiagnostics('ruff', monaco.Uri.parse('file:///a(1).py').toString())).toHaveLength(1)
    })
})
