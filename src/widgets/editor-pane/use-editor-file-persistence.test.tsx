import { afterEach, describe, expect, mock, test } from 'bun:test'
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

/**
 * The disk write, made observable *and* controllable: the "save reports completion" cases below turn
 * on when the `file_save` round trip settles, not just on whether it was issued (audit wave 2 #8).
 * `respond` is what the stubbed IPC returns, so a case can hold the write open or fail it.
 */
const fileSaves = { writes: [] as { path: string; content: string }[], respond: (): Promise<null> => Promise.resolve(null) }

mock.module('@entities/file/file.ipc', () => ({
    openFile: () => Promise.resolve(null),
    saveFile: (input: { path: string; content: string }) => {
        fileSaves.writes.push(input)
        return fileSaves.respond()
    },
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
    sourceMissing: false,
})

const createFakeEditor = (model: ReturnType<typeof createFakeModel>) =>
    ({
        getModel: () => model,
        saveViewState: () => null,
        restoreViewState: () => {},
    }) as unknown as NonNullable<PersistenceInput['editor']>

type MountOptions = { path: string; hadLiveModel: boolean; mirror: MirrorEntry | null; autoSaveDelayMs?: number }

/**
 * Mounts the hook the way `EditorPane` does, in the order React actually produces: `editor` is `null`
 * on the first commit (the parent only learns of the instance from `CodeEditor`'s own mount
 * callback), and `CodeEditor`'s model-attach effect — a CHILD effect, so it always runs before the
 * parent's — reports the attach through `noteModelAttach` before that second commit. Getting this
 * order right is the point: a test that handed the hook a live `editor` from the very first render
 * would never exercise the mount reconciliation at all.
 */
const mountPaneOnLiveModel = async ({ path, hadLiveModel, mirror, autoSaveDelayMs = 0 }: MountOptions) => {
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
        autoSaveDelayMs,
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
    const { result, rerender, unmount } = renderHookWithProviders(
        ({ liveEditor }: { liveEditor: PersistenceInput['editor'] }) => useEditorFilePersistence(buildInput(liveEditor)),
        { queryClient, initialProps },
    )

    act(() => result.current.noteModelAttach({ hadLiveModel }))
    await act(async () => {
        rerender({ liveEditor: editor })
    })

    return { result, model, tabDirtyCalls, unmount }
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

const EDITED_CONTENT = 'edited in the pane\n'
const WRITE_FAILED = new Error('disk is full')

/** Lets the microtask chain inside `handleSave` (code actions, format, cleanup, the mutation's own retryer hop) run without resolving the write itself. */
const flushPendingSaveSteps = async () => {
    for (let hop = 0; hop < 6; hop += 1) await new Promise((resolve) => setTimeout(resolve, 0))
}

/**
 * `handleSave` answers whether the file is now on disk, and answers it only once the write has
 * settled — the guarantee the close confirmation's "Save" stands on (audit wave 2 #8). It used to
 * fire `useSaveFile`'s `mutate` and return immediately, so awaiting it proved nothing at all.
 */
describe('useEditorFilePersistence 저장 완료 보고 (감사 웨이브 2 #8)', () => {
    afterEach(() => {
        fileSaves.respond = () => Promise.resolve(null)
        fileSaves.writes.length = 0
    })

    test('저장 중 추가 입력은 다음 자동 저장으로 이어지고 최신 초안까지 저장된다', async () => {
        const AUTO_SAVE_DELAY_MS = 20
        const AUTO_SAVE_SETTLE_MS = 100
        const path = '/repo/auto-save-followup.ts'
        const { result } = await mountPaneOnLiveModel({ path, hadLiveModel: false, mirror: null, autoSaveDelayMs: AUTO_SAVE_DELAY_MS })
        const write = Promise.withResolvers<null>()
        fileSaves.respond = () => write.promise
        act(() => result.current.handleChange(() => 'first'))
        const saving = result.current.handleSave()
        await act(flushPendingSaveSteps)
        expect(fileSaves.writes).toEqual([{ path, content: 'first' }])
        act(() => result.current.handleChange(() => 'latest'))
        fileSaves.respond = () => Promise.resolve(null)
        await act(async () => {
            write.resolve(null)
            expect(await saving).toBe(false)
        })
        await act(async () => {
            await new Promise((resolve) => setTimeout(resolve, AUTO_SAVE_SETTLE_MS))
        })
        expect(fileSaves.writes).toEqual([
            { path, content: 'first' },
            { path, content: 'latest' },
        ])
        expect(result.current.dirty).toBe(false)
    })

    test('저장 대기 중 닫힌 pane 은 완료 후 자동 저장을 예약하지 않는다', async () => {
        const AUTO_SAVE_DELAY_MS = 20
        const AUTO_SAVE_SETTLE_MS = 100
        const path = '/repo/auto-save-unmounted.ts'
        const { result, unmount } = await mountPaneOnLiveModel({ path, hadLiveModel: false, mirror: null, autoSaveDelayMs: AUTO_SAVE_DELAY_MS })
        const write = Promise.withResolvers<null>()
        fileSaves.respond = () => write.promise
        act(() => result.current.handleChange(() => 'first'))
        const saving = result.current.handleSave()
        await act(flushPendingSaveSteps)
        act(() => result.current.handleChange(() => 'latest'))
        unmount()
        await act(async () => {
            write.resolve(null)
            expect(await saving).toBe(false)
            await new Promise((resolve) => setTimeout(resolve, AUTO_SAVE_SETTLE_MS))
        })
        expect(fileSaves.writes).toEqual([{ path, content: 'first' }])
    })

    test('handleSave 는 디스크 쓰기가 끝나기 전에는 resolve 하지 않고, 끝나면 성공을 보고한다', async () => {
        const path = '/repo/save-awaits.ts'
        const { result } = await mountPaneOnLiveModel({ path, hadLiveModel: false, mirror: null })
        act(() => result.current.handleChange(() => EDITED_CONTENT))

        const write: { release: (() => void) | null } = { release: null }
        fileSaves.respond = () =>
            new Promise<null>((resolve) => {
                write.release = () => resolve(null)
            })

        const outcome: { current: boolean | null } = { current: null }
        const saving = (async () => {
            outcome.current = await result.current.handleSave()
        })()

        await act(async () => {
            await flushPendingSaveSteps()
        })

        expect(fileSaves.writes).toEqual([{ path, content: EDITED_CONTENT }])
        expect(outcome.current).toBeNull()

        await act(async () => {
            write.release?.()
            await saving
        })

        expect(outcome.current).toBe(true)
    })

    test('마운트된 pane 은 자기 저장 파이프라인을 tabId 로 등록한다 — 닫기 확인이 그것으로 저장한다', async () => {
        const { getSaveRequest } = await import('@entities/editor/save-request-registry')
        const path = '/repo/registered-save.ts'
        const { result } = await mountPaneOnLiveModel({ path, hadLiveModel: false, mirror: null })
        act(() => result.current.handleChange(() => EDITED_CONTENT))

        const requestSave = getSaveRequest(TAB_ID)
        expect(requestSave).not.toBeNull()

        const outcome: { current: boolean | null } = { current: null }
        await act(async () => {
            outcome.current = (await requestSave?.()) ?? null
        })

        expect(outcome.current).toBe(true)
        expect(fileSaves.writes).toEqual([{ path, content: EDITED_CONTENT }])
    })

    test('쓰기가 실패하면 실패를 보고한다 — 닫기 확인이 그 답으로 닫기를 취소한다', async () => {
        const path = '/repo/save-fails.ts'
        const { result } = await mountPaneOnLiveModel({ path, hadLiveModel: false, mirror: null })
        act(() => result.current.handleChange(() => EDITED_CONTENT))

        fileSaves.respond = () => Promise.reject(WRITE_FAILED)

        const outcome: { current: boolean | null } = { current: null }
        await act(async () => {
            outcome.current = await result.current.handleSave()
        })

        expect(fileSaves.writes).toEqual([{ path, content: EDITED_CONTENT }])
        expect(outcome.current).toBe(false)
    })
})
