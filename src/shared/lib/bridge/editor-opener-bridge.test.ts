import { describe, expect, test } from 'bun:test'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import {
    type OpenFileFromEditorRequest,
    registerLspEditorOpener,
    requestOpenFileFromEditor,
    subscribeOpenFileFromEditor,
} from '@shared/lib/bridge/editor-opener-bridge'

describe('editorOpenerBridge — pub/sub', () => {
    test('구독한 리스너는 요청을 그대로 전달받는다', () => {
        let received: OpenFileFromEditorRequest | undefined
        const unsubscribe = subscribeOpenFileFromEditor((request) => (received = request))

        requestOpenFileFromEditor({ path: '/workspace/a.ts', line: 3, column: 5 })
        unsubscribe()

        expect(received).toEqual({ path: '/workspace/a.ts', line: 3, column: 5 })
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenFileFromEditor(() => (calls += 1))
        unsubscribe()

        requestOpenFileFromEditor({ path: '/workspace/a.ts', line: 1, column: 1 })

        expect(calls).toBe(0)
    })

    test('여러 리스너가 모두 호출된다', () => {
        let first = 0
        let second = 0
        const unsubscribeFirst = subscribeOpenFileFromEditor(() => (first += 1))
        const unsubscribeSecond = subscribeOpenFileFromEditor(() => (second += 1))

        requestOpenFileFromEditor({ path: '/workspace/a.ts', line: 1, column: 1 })
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe(1)
        expect(second).toBe(1)
    })
})

const createFakeUri = (raw: string) => {
    const schemeMatch = /^([a-zA-Z][a-zA-Z\d+.-]*):(\/\/)?(.*)$/.exec(raw)
    const scheme = schemeMatch ? schemeMatch[1] : 'file'
    const hasAuthority = !!schemeMatch?.[2]
    const fsPath = schemeMatch ? schemeMatch[3] : raw
    return { scheme, fsPath, toString: () => `${scheme}:${hasAuthority ? '//' : ''}${fsPath}` }
}

type FakeUri = ReturnType<typeof createFakeUri>
type FakeEditor = {
    getModel: () => { uri: FakeUri } | null
    setPosition: (position: { lineNumber: number; column: number }) => void
    revealPositionInCenter: (position: { lineNumber: number; column: number }) => void
    focus: () => void
}
type CapturedOpener = { openCodeEditor: (source: FakeEditor, resource: FakeUri, selectionOrPosition?: unknown) => boolean | Promise<boolean> }

const createFakeEditor = (uri: FakeUri | null): FakeEditor & { setPositionCalls: unknown[]; revealCalls: unknown[]; focusCalls: number } => {
    const editor = {
        setPositionCalls: [] as unknown[],
        revealCalls: [] as unknown[],
        focusCalls: 0,
        getModel: () => (uri ? { uri } : null),
        setPosition: (position: { lineNumber: number; column: number }) => editor.setPositionCalls.push(position),
        revealPositionInCenter: (position: { lineNumber: number; column: number }) => editor.revealCalls.push(position),
        focus: () => (editor.focusCalls += 1),
    }
    return editor
}

const createFakeMonaco = (editors: FakeEditor[] = []) => {
    let captured: CapturedOpener | undefined
    const fakeMonaco = {
        editor: {
            registerEditorOpener: (opener: CapturedOpener) => {
                captured = opener
                return { dispose: () => {} }
            },
            getEditors: () => editors,
        },
    }
    return { monaco: fakeMonaco as unknown as Monaco, getOpener: () => captured }
}

describe('registerLspEditorOpener', () => {
    test('target 이 source 자신의 모델과 같으면 false 를 반환해 monaco 기본 동작에 맡긴다', () => {
        const uri = createFakeUri('file:///workspace/a.ts')
        const source = createFakeEditor(uri)
        const { monaco, getOpener } = createFakeMonaco()
        registerLspEditorOpener(monaco)

        const handled = getOpener()?.openCodeEditor(source, uri)

        expect(handled).toBe(false)
    })

    test('file scheme 타깃은 구독자가 없으면 false 를 반환하고 요청을 보내지 않는다', () => {
        const source = createFakeEditor(createFakeUri('file:///workspace/source.ts'))
        const target = createFakeUri('file:///workspace/target.ts')
        const { monaco, getOpener } = createFakeMonaco()
        registerLspEditorOpener(monaco)

        const handled = getOpener()?.openCodeEditor(source, target)

        expect(handled).toBe(false)
    })

    test('file scheme 타깃은 구독자가 있으면 path/line/column 을 실어 요청을 보내고 true 를 반환한다', () => {
        const source = createFakeEditor(createFakeUri('file:///workspace/source.ts'))
        const target = createFakeUri('file:///workspace/target.ts')
        const { monaco, getOpener } = createFakeMonaco()
        registerLspEditorOpener(monaco)

        let received: OpenFileFromEditorRequest | undefined
        const unsubscribe = subscribeOpenFileFromEditor((request) => (received = request))
        const handled = getOpener()?.openCodeEditor(source, target, { startLineNumber: 4, startColumn: 2, endLineNumber: 4, endColumn: 6 })
        unsubscribe()

        expect(handled).toBe(true)
        expect(received).toEqual({ path: '/workspace/target.ts', line: 4, column: 2 })
    })

    test('selectionOrPosition 이 IPosition 이면 lineNumber/column 을 그대로 쓴다', () => {
        const source = createFakeEditor(createFakeUri('file:///workspace/source.ts'))
        const target = createFakeUri('file:///workspace/target.ts')
        const { monaco, getOpener } = createFakeMonaco()
        registerLspEditorOpener(monaco)

        let received: OpenFileFromEditorRequest | undefined
        const unsubscribe = subscribeOpenFileFromEditor((request) => (received = request))
        getOpener()?.openCodeEditor(source, target, { lineNumber: 9, column: 3 })
        unsubscribe()

        expect(received).toEqual({ path: '/workspace/target.ts', line: 9, column: 3 })
    })

    test('selectionOrPosition 이 없으면 1,1 로 기본값 처리한다', () => {
        const source = createFakeEditor(createFakeUri('file:///workspace/source.ts'))
        const target = createFakeUri('file:///workspace/target.ts')
        const { monaco, getOpener } = createFakeMonaco()
        registerLspEditorOpener(monaco)

        let received: OpenFileFromEditorRequest | undefined
        const unsubscribe = subscribeOpenFileFromEditor((request) => (received = request))
        getOpener()?.openCodeEditor(source, target, undefined)
        unsubscribe()

        expect(received).toEqual({ path: '/workspace/target.ts', line: 1, column: 1 })
    })

    test('file/untitled 이외의 scheme 은 false 를 반환한다', () => {
        const source = createFakeEditor(createFakeUri('file:///workspace/source.ts'))
        const target = createFakeUri('https://example.com/readme')
        const { monaco, getOpener } = createFakeMonaco()
        registerLspEditorOpener(monaco)

        const unsubscribe = subscribeOpenFileFromEditor(() => {})
        const handled = getOpener()?.openCodeEditor(source, target)
        unsubscribe()

        expect(handled).toBe(false)
    })

    test('untitled scheme 타깃이 다른 패널에 열려 있으면 해당 에디터에서 직접 revealposition 한다', () => {
        const source = createFakeEditor(createFakeUri('untitled:tab-1'))
        const target = createFakeUri('untitled:tab-2')
        const targetEditor = createFakeEditor(target)
        const { monaco, getOpener } = createFakeMonaco([source, targetEditor])
        registerLspEditorOpener(monaco)

        const handled = getOpener()?.openCodeEditor(source, target, { lineNumber: 2, column: 4 })

        expect(handled).toBe(true)
        expect(targetEditor.setPositionCalls).toEqual([{ lineNumber: 2, column: 4 }])
        expect(targetEditor.revealCalls).toEqual([{ lineNumber: 2, column: 4 }])
        expect(targetEditor.focusCalls).toBe(1)
    })

    test('untitled scheme 타깃을 표시하는 에디터가 없으면 false 를 반환한다 (열지 못함)', () => {
        const source = createFakeEditor(createFakeUri('untitled:tab-1'))
        const target = createFakeUri('untitled:tab-99')
        const { monaco, getOpener } = createFakeMonaco([source])
        registerLspEditorOpener(monaco)

        const handled = getOpener()?.openCodeEditor(source, target)

        expect(handled).toBe(false)
    })
})
