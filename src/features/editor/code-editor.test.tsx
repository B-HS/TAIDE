import { describe, expect, mock, test } from 'bun:test'
import type { ComponentProps } from 'react'
import { renderWithProviders } from '@shared/testing/render'

/**
 * `code-editor.tsx` statically imports `@shared/lib/monaco/setup`, whose real module pulls in
 * monaco's `?worker` bundles that only Vite can resolve — the same reason every other suite in this
 * repo stubs that module and reaches the component under test through a *dynamic* `import()`
 * (`claude-diff-pane.test.ts`, `pane-node-view-welcome.test.tsx`). The stub covers what the real
 * attach path touches — `editor.create`, the model factory `entities/editor/model-registry` calls for
 * real, and the `KeyMod`/`KeyCode` constants `attachAiInlineEditAction` reads — plus the marker and
 * keybinding members the other suites' own stubs declare, since `mock.module` is process-wide and
 * last-wins (`docs/memory/test-conventions.md` §3): a narrower stub registered here would be the one
 * a later file's lazy import of this module sees.
 *
 * What this covers is exactly the focus decision (`autoFocus` × path-change), which is pure
 * bookkeeping around a single `editor.focus()` call and therefore survives the stub intact. Monaco's
 * own behaviour — that `focus()` actually moves the caret, that `restoreViewState` puts it back where
 * it was — stays an e2e concern (`docs/memory/test-conventions.md` §5).
 */
const focusCalls = { count: 0 }

const createFakeModel = (content: string, languageId: string) => ({
    isDisposed: () => false,
    getValue: () => content,
    getLanguageId: () => languageId,
    onDidChangeContent: () => ({ dispose: () => {} }),
    updateOptions: () => {},
    dispose: () => {},
})

const createFakeEditor = () => ({
    addAction: () => ({ dispose: () => {} }),
    onDidChangeModelContent: () => ({ dispose: () => {} }),
    onDidChangeCursorPosition: () => ({ dispose: () => {} }),
    onDidChangeModel: () => ({ dispose: () => {} }),
    updateOptions: () => {},
    setModel: () => {},
    getModel: () => null,
    saveViewState: () => null,
    restoreViewState: () => {},
    focus: () => {
        focusCalls.count += 1
    },
    dispose: () => {},
})

mock.module('@shared/lib/monaco/setup', () => ({
    monaco: {
        Uri: { file: (path: string) => ({ toString: () => `file://${path}` }), parse: (path: string) => ({ toString: () => path }) },
        editor: {
            create: () => createFakeEditor(),
            createModel: (content: string, languageId: string) => createFakeModel(content, languageId),
            createDiffEditor: () => ({ setModel: () => {}, dispose: () => {} }),
            getModel: () => null,
            getEditors: () => [],
            setModelLanguage: () => {},
            onDidCreateModel: () => ({ dispose: () => {} }),
            getModelMarkers: () => [],
            onDidChangeMarkers: () => ({ dispose: () => {} }),
            addKeybindingRules: () => ({ dispose: () => {} }),
        },
        MarkerSeverity: { Hint: 1, Info: 2, Warning: 4, Error: 8 },
        KeyMod: { CtrlCmd: 2048, Shift: 1024, Alt: 512, WinCtrl: 256 },
        KeyCode: { KeyI: 39 },
        languages: { InlineCompletionTriggerKind: { Automatic: 0, Explicit: 1 } },
    },
}))

const importCodeEditor = () => import('@features/editor/code-editor')

type CodeEditorProps = ComponentProps<Awaited<ReturnType<typeof importCodeEditor>>['CodeEditor']>

const buildProps = (overrides: Partial<CodeEditorProps>): CodeEditorProps => ({
    path: '/tmp/taide-code-editor-focus/a.ts',
    language: 'typescript',
    value: 'const a = 1\n',
    readOnly: false,
    largeFile: false,
    minimap: false,
    fontFamily: 'monospace',
    fontSize: 12,
    wordWrap: false,
    lineNumbers: true,
    tabSize: 4,
    insertSpaces: true,
    detectIndentation: false,
    renderWhitespace: 'none',
    bracketPairColorization: false,
    fontLigatures: false,
    cursorStyle: 'line',
    cursorBlinking: 'blink',
    scrollBeyondLastLine: false,
    stickyScroll: false,
    bracketPairGuides: false,
    smoothScrolling: false,
    cursorSmoothCaretAnimation: false,
    suggestPreview: false,
    rulers: [],
    formatOnType: false,
    formatOnPaste: false,
    aiAutoTabEnabled: false,
    aiCompletionConfig: null,
    onChange: () => {},
    onSave: () => {},
    onCursorLineChange: () => {},
    onMinimapToggle: () => {},
    ...overrides,
})

const renderCodeEditor = async (overrides: Partial<CodeEditorProps>) => {
    const { CodeEditor } = await importCodeEditor()
    focusCalls.count = 0
    const result = renderWithProviders(<CodeEditor {...buildProps(overrides)} />)

    return {
        rerender: (next: Partial<CodeEditorProps>) => result.rerender(<CodeEditor {...buildProps({ ...overrides, ...next })} />),
    }
}

describe('CodeEditor 모델 attach 포커스', () => {
    test('포커스된 pane 이면 모델을 붙이며 포커스를 가져간다', async () => {
        await renderCodeEditor({ path: '/tmp/taide-code-editor-focus/focused.ts', autoFocus: true })

        expect(focusCalls.count).toBe(1)
    })

    test('포커스되지 않은 pane 이면 포커스를 가져가지 않는다', async () => {
        await renderCodeEditor({ path: '/tmp/taide-code-editor-focus/unfocused.ts', autoFocus: false })

        expect(focusCalls.count).toBe(0)
    })

    test('autoFocus 를 넘기지 않는 호스트는 기존대로 포커스를 가져간다', async () => {
        await renderCodeEditor({ path: '/tmp/taide-code-editor-focus/default.ts' })

        expect(focusCalls.count).toBe(1)
    })

    test('같은 pane 안에서 탭이 바뀌면 다시 포커스를 가져간다', async () => {
        const { rerender } = await renderCodeEditor({ path: '/tmp/taide-code-editor-focus/switch-a.ts', autoFocus: true })

        rerender({ path: '/tmp/taide-code-editor-focus/switch-b.ts' })

        expect(focusCalls.count).toBe(2)
    })

    test('path 는 그대로고 language 만 바뀌면 포커스를 가져가지 않는다', async () => {
        const { rerender } = await renderCodeEditor({ path: '/tmp/taide-code-editor-focus/relanguage.ts', autoFocus: true })

        rerender({ language: 'javascript' })

        expect(focusCalls.count).toBe(1)
    })
})

/**
 * `onModelAttach` is the one fact only this component can report (d-66, audit §2-7/§2-8): once
 * `getOrCreateModel` has run, every later `getModel(path)` says "exists", so a host asking the
 * question from its own effects can no longer tell an inherited buffer from one this attach just
 * created. Exercised against the REAL `entities/editor/model-registry` (only monaco itself is
 * stubbed), so "already existed" means what it means in the app: a model another pane registered
 * for the same path.
 */
describe('CodeEditor onModelAttach (물려받은 모델인지 보고)', () => {
    test('이 attach 가 모델을 새로 만들었으면 hadLiveModel 은 false 다', async () => {
        const attaches: { path: string; hadLiveModel: boolean }[] = []

        await renderCodeEditor({ path: '/tmp/taide-code-editor-attach/fresh.ts', onModelAttach: (attach) => attaches.push(attach) })

        expect(attaches).toEqual([{ path: '/tmp/taide-code-editor-attach/fresh.ts', hadLiveModel: false }])
    })

    test('다른 pane 이 이미 연 경로에 붙으면 hadLiveModel 은 true 다 (스플릿 뷰의 두 번째 pane)', async () => {
        const path = '/tmp/taide-code-editor-attach/shared.ts'
        const { getOrCreateModel } = await import('@entities/editor/model-registry')
        getOrCreateModel(path, 'const a = 1\n', 'typescript')
        const attaches: { path: string; hadLiveModel: boolean }[] = []

        await renderCodeEditor({ path, onModelAttach: (attach) => attaches.push(attach) })

        expect(attaches).toEqual([{ path, hadLiveModel: true }])
    })

    test('같은 pane 안에서 탭을 바꾸면 새 경로로 다시 보고한다', async () => {
        const attaches: { path: string; hadLiveModel: boolean }[] = []
        const { rerender } = await renderCodeEditor({
            path: '/tmp/taide-code-editor-attach/switch-a.ts',
            onModelAttach: (attach) => attaches.push(attach),
        })

        rerender({ path: '/tmp/taide-code-editor-attach/switch-b.ts' })

        expect(attaches.map((attach) => attach.path)).toEqual([
            '/tmp/taide-code-editor-attach/switch-a.ts',
            '/tmp/taide-code-editor-attach/switch-b.ts',
        ])
    })

    test('path 는 그대로고 language 만 바뀐 재attach 는 보고하지 않는다 (같은 버퍼를 다시 붙이는 것뿐)', async () => {
        const attaches: { path: string; hadLiveModel: boolean }[] = []
        const { rerender } = await renderCodeEditor({
            path: '/tmp/taide-code-editor-attach/relanguage.ts',
            onModelAttach: (attach) => attaches.push(attach),
        })

        rerender({ language: 'javascript' })

        expect(attaches).toHaveLength(1)
    })
})
