import { describe, expect, mock, test } from 'bun:test'
import type { MirrorEntry, OpenedFile, ProjectId, TabId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderHookWithProviders } from '@shared/testing/render'

/**
 * A mutable stand-in for `ITextModel`: `entities/editor/model-registry.ts` (used here for real, so
 * two panes really do share one buffer the way they do in the app) hands these out and
 * `applyExternalContent` writes through them, so unlike the frozen fake in `code-editor.test.tsx`
 * this one has to let `setValue` actually change what `getValue` reports — that difference IS the
 * assertion in every test below.
 */
const createFakeModel = (content: string, languageId: string) => {
    let value = content
    let disposed = false
    return {
        getValue: () => value,
        setValue: (next: string) => {
            value = next
        },
        getLanguageId: () => languageId,
        isDisposed: () => disposed,
        dispose: () => {
            disposed = true
        },
        onDidChangeContent: () => ({ dispose: () => {} }),
        updateOptions: () => {},
    }
}

/**
 * The monaco surface `model-registry` and this hook's `applyExternalContent` path touch. Registered
 * process-wide and last-wins (`docs/memory/test-conventions.md` §3), so it declares the same member
 * set the other suites' stubs do rather than only what this file reaches.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: {
        Uri: { file: (path: string) => ({ toString: () => `file://${path}` }), parse: (path: string) => ({ toString: () => path }) },
        editor: {
            create: () => ({ dispose: () => {} }),
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

const mirrorCalls = { writes: [] as { path: string; content: string }[], cleared: [] as string[] }

mock.module('@entities/file/file.ipc', () => ({
    openFile: () => Promise.resolve(null),
    saveFile: () => Promise.resolve(null),
    createEntry: () => Promise.resolve(null),
    renameEntry: () => Promise.resolve(null),
    deleteEntry: () => Promise.resolve(null),
    copyEntry: () => Promise.resolve(null),
    mirrorDirty: (input: { path: string; content: string }) => {
        mirrorCalls.writes.push({ path: input.path, content: input.content })
        return Promise.resolve(1_700_000_000_000)
    },
    listMirrors: () => Promise.resolve([]),
    clearMirror: (input: { path: string }) => {
        mirrorCalls.cleared.push(input.path)
        return Promise.resolve(null)
    },
    pruneMirrors: () => Promise.resolve(null),
    mirrorUntitled: () => Promise.resolve(null),
    listUntitledMirrors: () => Promise.resolve([]),
    clearUntitledMirror: () => Promise.resolve(null),
    pruneUntitledMirrors: () => Promise.resolve(null),
    flushMirrorsComplete: () => Promise.resolve(null),
}))

const importPersistence = () => import('@widgets/editor-pane/use-editor-file-persistence')
const importModelRegistry = () => import('@entities/editor/model-registry')

type PersistenceInput = Parameters<Awaited<ReturnType<typeof importPersistence>>['useEditorFilePersistence']>[0]

const PROJECT_ID = 'project-1' as ProjectId
const TAB_ID = 'tab-1' as TabId
const DISK_CONTENT = 'on disk\n'

const buildOpenedFile = (path: string, content: string): OpenedFile => ({
    path,
    content,
    languageId: 'typescript',
    byteSize: content.length,
    lineCount: 1,
    tier: 'normal',
    readOnly: false,
    encodingLossy: false,
    modifiedMs: 1_700_000_000_000,
    editorConfig: { indentStyle: null, indentSize: null, tabWidth: null, insertFinalNewline: null, trimTrailingWhitespace: null },
})

const buildMirror = (path: string, content: string): MirrorEntry => ({
    path,
    content,
    savedAtMs: 1_700_000_000_000,
    diskModifiedMs: 1_700_000_000_000,
    conflict: false,
})

const createFakeEditor = (model: ReturnType<typeof createFakeModel>) =>
    ({
        getModel: () => model,
        saveViewState: () => null,
        restoreViewState: () => {},
    }) as unknown as NonNullable<PersistenceInput['editor']>

type MountOptions = { path: string; hadLiveModel: boolean; mirror: MirrorEntry | null }

/**
 * Mounts the hook the way `EditorPane` does, in the order React actually produces: `editor` is `null`
 * on the first commit (the parent only learns of the instance from `CodeEditor`'s own mount
 * callback), and `CodeEditor`'s model-attach effect — a CHILD effect, so it always runs before the
 * parent's — reports the attach through `noteModelAttach` before that second commit. Getting this
 * order right is the point: a test that handed the hook a live `editor` from the very first render
 * would never exercise the mount reconciliation at all.
 */
const mountPaneOnLiveModel = async ({ path, hadLiveModel, mirror }: MountOptions) => {
    const { useEditorFilePersistence } = await importPersistence()
    const { getOrCreateModel } = await importModelRegistry()

    const model = getOrCreateModel(path, DISK_CONTENT, 'typescript') as unknown as ReturnType<typeof createFakeModel>
    const editor = createFakeEditor(model)
    const tabDirtyCalls: { tabId: TabId; dirty: boolean }[] = []

    const queryClient = createTestQueryClient()
    queryClient.setQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(PROJECT_ID), mirror ? [mirror] : [])

    const buildInput = (liveEditor: PersistenceInput['editor']): PersistenceInput => ({
        projectId: PROJECT_ID,
        path,
        tabId: TAB_ID,
        file: buildOpenedFile(path, DISK_CONTENT),
        autoSaveDelayMs: 0,
        formatOnSave: false,
        trimTrailingWhitespaceOnSave: false,
        insertFinalNewlineOnSave: false,
        isMarkdown: false,
        isOutsideProjectRoot: false,
        editor: liveEditor,
        setSyncedContent: () => {},
        setTabDirty: ((variables: { tabId: TabId; dirty: boolean }) => {
            tabDirtyCalls.push(variables)
        }) as PersistenceInput['setTabDirty'],
        notifyLspSessionsOfSave: () => Promise.resolve(),
        runCodeActionsOnSave: () => Promise.resolve(),
        previewTimeoutRef: { current: undefined },
        setPreviewSource: () => {},
        t: ((key: string) => key) as PersistenceInput['t'],
    })

    const initialProps: { liveEditor: PersistenceInput['editor'] } = { liveEditor: null }
    const { result, rerender } = renderHookWithProviders(
        ({ liveEditor }: { liveEditor: PersistenceInput['editor'] }) => useEditorFilePersistence(buildInput(liveEditor)),
        { queryClient, initialProps },
    )

    act(() => result.current.noteModelAttach({ hadLiveModel }))
    await act(async () => {
        rerender({ liveEditor: editor })
    })

    return { result, model, tabDirtyCalls }
}

describe('useEditorFilePersistence 마운트 시 라이브 모델 인수 (감사 §2-7 · §2-8)', () => {
    test('형제 pane 이 편집 중인 모델 위에 마운트하면 미러로 되돌리지 않고 현재 값을 draft 로 인수한다', async () => {
        const path = '/repo/live-sibling.ts'
        const { getOrCreateModel } = await importModelRegistry()
        const sharedModel = getOrCreateModel(path, DISK_CONTENT, 'typescript') as unknown as ReturnType<typeof createFakeModel>
        sharedModel.setValue('typed in the other pane\n')

        const { result, model, tabDirtyCalls } = await mountPaneOnLiveModel({
            path,
            hadLiveModel: true,
            mirror: buildMirror(path, 'stale mirror snapshot\n'),
        })

        expect(model.getValue()).toBe('typed in the other pane\n')
        expect(result.current.dirty).toBe(true)
        expect(result.current.restoreNotice).toBe('none')
        expect(tabDirtyCalls).toEqual([{ tabId: TAB_ID, dirty: true }])
    })

    test('미러가 없어도 라이브 모델의 미관측 편집을 dirty 로 인수한다 (디스크 sync 가 덮어쓰지 못하게)', async () => {
        const path = '/repo/live-no-mirror.ts'
        const { getOrCreateModel } = await importModelRegistry()
        const sharedModel = getOrCreateModel(path, DISK_CONTENT, 'typescript') as unknown as ReturnType<typeof createFakeModel>
        sharedModel.setValue('unmirrored sibling edit\n')

        const { result, model } = await mountPaneOnLiveModel({ path, hadLiveModel: true, mirror: null })

        expect(model.getValue()).toBe('unmirrored sibling edit\n')
        expect(result.current.dirty).toBe(true)
        expect(result.current.isDraftDirty()).toBe(true)
    })

    test('크래시 후 첫 마운트(라이브 모델 없음)는 종전대로 미러를 복원하고 배너를 띄운다', async () => {
        const path = '/repo/crash-restore.ts'

        const { result, model, tabDirtyCalls } = await mountPaneOnLiveModel({
            path,
            hadLiveModel: false,
            mirror: buildMirror(path, 'recovered unsaved work\n'),
        })

        expect(model.getValue()).toBe('recovered unsaved work\n')
        expect(result.current.restoreNotice).toBe('mirrorRestored')
        expect(result.current.dirty).toBe(true)
        expect(tabDirtyCalls).toEqual([{ tabId: TAB_ID, dirty: true }])
    })

    test('아무도 편집하지 않은 파일을 두 번째 pane 에 열면(모델 = 디스크) 인수하지 않고 clean 으로 남는다', async () => {
        const path = '/repo/live-clean.ts'

        const { result, model, tabDirtyCalls } = await mountPaneOnLiveModel({ path, hadLiveModel: true, mirror: null })

        expect(model.getValue()).toBe(DISK_CONTENT)
        expect(result.current.dirty).toBe(false)
        expect(result.current.restoreNotice).toBe('none')
        expect(tabDirtyCalls).toEqual([])
    })

    test('라이브 모델이었어도 디스크와 같으면 미러 복원은 그대로 동작한다 (인수가 복원 경로를 통째로 가리지 않는다)', async () => {
        const path = '/repo/live-clean-with-mirror.ts'

        const { result, model } = await mountPaneOnLiveModel({
            path,
            hadLiveModel: true,
            mirror: buildMirror(path, 'mirror for an untouched buffer\n'),
        })

        expect(model.getValue()).toBe('mirror for an untouched buffer\n')
        expect(result.current.restoreNotice).toBe('mirrorRestored')
    })
})
