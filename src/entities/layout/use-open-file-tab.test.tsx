import { describe, expect, mock, test } from 'bun:test'
import type { PaneNode, ProjectLayout } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { QUERY_KEY } from '@shared/constants/query-key'
import { IpcError } from '@shared/api/unwrap-result'
import { createTestQueryClient, renderHookWithProviders, waitFor } from '@shared/testing/render'
import { registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { consumePendingReveal } from '@entities/editor/reveal-registry'

/**
 * `useOpenFileTab` is the single entry point every file open funnels through (batch 3 contract
 * §A.2 item 4), and the branch worth locking is the quick-open index repair: a `NotFound` means the
 * palette's `SEARCH.PROJECT_FILES` snapshot handed out a path that is no longer on disk, so that
 * listing — and only that project's — is invalidated, while any other failure says nothing about
 * the index and must leave it alone.
 *
 * The second branch is the `null` target: `layout_open_tab` would fall back to the *main* tree's
 * `focused_pane`, so `withCurrentWindowTarget` resolves it against this window's tree before the
 * call goes out (d-62 §1.D). The harness always resolves to the main window (`location` carries no
 * query string — `docs/memory/test-conventions.md` §4), which is exactly the no-regression case
 * worth locking here; the auxiliary branch is `resolveWindowPaneTree`'s own unit under test. That
 * resolution also has to survive a *stale* cached `focusedPane` (d-65 §1.F1) — promoting a pruned
 * pane id to an explicit `target` is precisely what turned a closed split half into `pane_not_found`
 * on every subsequent open — so the stale case is locked here at the call site, not just in the unit.
 *
 * `layout.query.ts` reaches monaco (through `tab-path-change.ts` → `model-registry`) and its own
 * `.ipc` module at import time, so both are stubbed before it is pulled in through a *dynamic*
 * `import()`. `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3), so the `layout.ipc` fake covers that module's *entire*
 * export surface and every entry defaults to the same rejection a real `invoke` produces with no
 * `window.__TAURI_INTERNALS__`; only `openTab` is swapped per test, through a mutable reference the
 * factory reads on each call.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

type OpenTabInput = { projectId: string; kind: { kind: string; path?: string }; title: string; target: string | null; preview: boolean }

const capturedOpenTabCalls: OpenTabInput[] = []
const openTabImpl = { current: rejectLikeUnavailableIpc as (input: OpenTabInput) => Promise<ProjectLayout> }

mock.module('@entities/layout/layout.ipc', () => ({
    getLayout: rejectLikeUnavailableIpc,
    openTab: (input: OpenTabInput) => {
        capturedOpenTabCalls.push(input)
        return openTabImpl.current(input)
    },
    closeTab: rejectLikeUnavailableIpc,
    activateTab: rejectLikeUnavailableIpc,
    moveTab: rejectLikeUnavailableIpc,
    splitPane: rejectLikeUnavailableIpc,
    openTabInSplit: rejectLikeUnavailableIpc,
    resizePane: rejectLikeUnavailableIpc,
    focusPane: rejectLikeUnavailableIpc,
    pinTab: rejectLikeUnavailableIpc,
    setTabPreview: rejectLikeUnavailableIpc,
    setTabDirty: rejectLikeUnavailableIpc,
    setTerminalSession: rejectLikeUnavailableIpc,
    reopenClosedTab: rejectLikeUnavailableIpc,
    openUntitledTab: rejectLikeUnavailableIpc,
    convertUntitledTab: rejectLikeUnavailableIpc,
    moveTabToWindow: rejectLikeUnavailableIpc,
    setShellView: rejectLikeUnavailableIpc,
    setTabViewState: rejectLikeUnavailableIpc,
    applyTabPathChange: rejectLikeUnavailableIpc,
}))

const importLayoutQuery = () => import('@entities/layout/layout.query')

const PROJECT_ID = 'project-1'
const OTHER_PROJECT_ID = 'project-2'
const FILE_PATH = '/project/src/app.tsx'

const EMPTY_LEAF: PaneNode = { node: 'leaf', id: 'leaf-1', tabs: [], active: null }

const OPENED_TAB_ID = 'tab-opened'
const OTHER_PANE_TAB_ID = 'tab-other-pane'

const fileTab = (id: string, path: string) => ({ id, kind: { kind: 'file', path } as const, title: 'app.tsx' })

/**
 * What `layout_open_tab` returns for a file opened into `leaf-1` while a *second* group already has
 * the same file open: both panes hold a `file` tab for `FILE_PATH`, and only the target pane's
 * `active` names the tab this open produced (audit #9 — the path-keyed predecessor moved whichever
 * editor monaco listed first, which is the other pane here).
 */
const buildOpenedLayout = (revision: number): ProjectLayout => ({
    version: 2,
    root: {
        node: 'split',
        id: 'split-1',
        dir: 'horizontal',
        sizes: [0.5, 0.5],
        children: [
            { node: 'leaf', id: 'leaf-1', tabs: [fileTab(OPENED_TAB_ID, FILE_PATH)], active: OPENED_TAB_ID },
            { node: 'leaf', id: 'leaf-2', tabs: [fileTab(OTHER_PANE_TAB_ID, FILE_PATH)], active: OTHER_PANE_TAB_ID },
        ],
    },
    focusedPane: 'leaf-1',
    revision,
})

type FakeEditor = { positions: { lineNumber: number; column: number }[]; focusCount: number }

const createFakeEditor = () => {
    const calls: FakeEditor = { positions: [], focusCount: 0 }
    const editor = {
        setPosition: (position: { lineNumber: number; column: number }) => calls.positions.push(position),
        revealPositionInCenter: () => undefined,
        focus: () => {
            calls.focusCount += 1
        },
    }
    return { calls, editor: editor as unknown as monaco.editor.IStandaloneCodeEditor }
}

const buildLayout = (revision: number): ProjectLayout => ({ version: 2, root: EMPTY_LEAF, focusedPane: 'leaf-1', revision })

/** A cached snapshot whose `focusedPane` points at a pane the tree no longer has — what a window holds between another pane's last tab closing and the `layout:changed` echo (d-65 §1.F1). */
const buildStaleFocusLayout = (revision: number): ProjectLayout => ({ version: 2, root: EMPTY_LEAF, focusedPane: 'closed-pane', revision })

const seedProjectFileIndex = async (queryClient: ReturnType<typeof createTestQueryClient>, projectId: string) => {
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId), queryFn: () => Promise.resolve([]), gcTime: Infinity })
}

const setupIndexes = async () => {
    const queryClient = createTestQueryClient()
    await seedProjectFileIndex(queryClient, PROJECT_ID)
    await seedProjectFileIndex(queryClient, OTHER_PROJECT_ID)
    return queryClient
}

const isIndexInvalidated = (queryClient: ReturnType<typeof createTestQueryClient>, projectId: string) =>
    queryClient.getQueryState(QUERY_KEY.SEARCH.PROJECT_FILES(projectId))?.isInvalidated

describe('useOpenFileTab', () => {
    test('파일 탭 kind 를 만들고 제목을 경로의 파일명으로 채워 target·preview 와 함께 넘긴다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.resolve(buildLayout(1))

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: true, target: 'leaf-9' })
        await waitFor(() => expect(capturedOpenTabCalls.length).toBeGreaterThan(0))

        expect(capturedOpenTabCalls.at(-1)).toEqual({
            projectId: PROJECT_ID,
            kind: { kind: 'file', path: FILE_PATH },
            title: 'app.tsx',
            target: 'leaf-9',
            preview: true,
        })
    })

    test('target 이 null 이면 이 창의 포커스 pane 으로 해소해 보낸다 (서버 폴백은 언제나 main 트리)', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        await queryClient.fetchQuery({
            queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID),
            queryFn: () => Promise.resolve(buildLayout(1)),
            gcTime: Infinity,
        })
        openTabImpl.current = () => Promise.resolve(buildLayout(2))
        const sentBefore = capturedOpenTabCalls.length

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: false, target: null })
        await waitFor(() => expect(capturedOpenTabCalls.length).toBeGreaterThan(sentBefore))

        expect(capturedOpenTabCalls.at(-1)?.target).toBe('leaf-1')
    })

    test('캐시의 focusedPane 이 이미 닫힌 pane 이면 stale id 대신 첫 leaf 를 target 으로 보낸다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        await queryClient.fetchQuery({
            queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID),
            queryFn: () => Promise.resolve(buildStaleFocusLayout(1)),
            gcTime: Infinity,
        })
        openTabImpl.current = () => Promise.resolve(buildLayout(2))
        const sentBefore = capturedOpenTabCalls.length

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: false, target: null })
        await waitFor(() => expect(capturedOpenTabCalls.length).toBeGreaterThan(sentBefore))

        expect(capturedOpenTabCalls.at(-1)?.target).toBe('leaf-1')
    })

    test('제목을 명시하면 파일명 대신 그 제목을 쓰고, 레이아웃 캐시가 비어 있으면 target 은 null 그대로 서버 폴백에 맡긴다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.resolve(buildLayout(1))

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: false, target: null, title: 'Custom' })
        await waitFor(() => expect(capturedOpenTabCalls.at(-1)?.title).toBe('Custom'))

        expect(capturedOpenTabCalls.at(-1)?.target).toBeNull()
    })

    test('성공하면 새 레이아웃을 캐시에 쓰고 onSuccess 콜백에 그대로 넘긴다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        await queryClient.fetchQuery({
            queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID),
            queryFn: () => Promise.resolve(buildLayout(1)),
            gcTime: Infinity,
        })
        const layout = buildLayout(7)
        openTabImpl.current = () => Promise.resolve(layout)
        const received: ProjectLayout[] = []

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: false, target: null }, { onSuccess: (value) => received.push(value) })
        await waitFor(() => expect(received.length).toBe(1))

        expect(received[0]).toBe(layout)
        expect(queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID))).toEqual(layout)
    })

    test('reveal 을 주면 이번에 연 탭의 에디터만 움직이고, 같은 파일을 연 다른 pane 의 에디터는 그대로 둔다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.resolve(buildOpenedLayout(3))
        const opened = createFakeEditor()
        const otherPane = createFakeEditor()
        registerEditorInstance(OPENED_TAB_ID, opened.editor)
        registerEditorInstance(OTHER_PANE_TAB_ID, otherPane.editor)

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: true, target: 'leaf-1', reveal: { line: 120, column: 7 } })
        await waitFor(() => expect(opened.calls.positions.length).toBe(1))

        expect(opened.calls.positions).toEqual([{ lineNumber: 120, column: 7 }])
        expect(otherPane.calls.positions).toEqual([])
        expect(otherPane.calls.focusCount).toBe(0)

        unregisterEditorInstance(OPENED_TAB_ID)
        unregisterEditorInstance(OTHER_PANE_TAB_ID)
    })

    test('연 탭이 아직 마운트되지 않았으면 그 탭이 마운트될 때 보류된 reveal 이 적용된다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.resolve(buildOpenedLayout(4))
        const received: ProjectLayout[] = []

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current(
            { projectId: PROJECT_ID, path: FILE_PATH, preview: true, target: 'leaf-1', reveal: { line: 12, column: 3 } },
            { onSuccess: (layout) => received.push(layout) },
        )
        await waitFor(() => expect(received.length).toBe(1))

        const mounted = createFakeEditor()
        consumePendingReveal(OPENED_TAB_ID, mounted.editor)
        expect(mounted.calls.positions).toEqual([{ lineNumber: 12, column: 3 }])
    })

    test('reveal 없이 열면 어떤 에디터의 커서도 건드리지 않는다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.resolve(buildOpenedLayout(5))
        const opened = createFakeEditor()
        registerEditorInstance(OPENED_TAB_ID, opened.editor)
        const received: ProjectLayout[] = []

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: true, target: 'leaf-1' }, { onSuccess: (layout) => received.push(layout) })
        await waitFor(() => expect(received.length).toBe(1))

        expect(opened.calls.positions).toEqual([])

        unregisterEditorInstance(OPENED_TAB_ID)
    })

    test('대상 pane 의 active 탭이 연 경로가 아니면 reveal 을 버린다 (엉뚱한 탭으로 새지 않는다)', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.resolve(buildOpenedLayout(6))
        const opened = createFakeEditor()
        registerEditorInstance(OPENED_TAB_ID, opened.editor)
        const received: ProjectLayout[] = []

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current(
            { projectId: PROJECT_ID, path: '/project/src/other.tsx', preview: true, target: 'leaf-1', reveal: { line: 5, column: 1 } },
            { onSuccess: (layout) => received.push(layout) },
        )
        await waitFor(() => expect(received.length).toBe(1))

        expect(opened.calls.positions).toEqual([])

        unregisterEditorInstance(OPENED_TAB_ID)
    })

    test('NotFound 실패면 그 프로젝트의 퀵오픈 인덱스만 무효화한다 (다른 프로젝트 인덱스는 그대로)', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.reject(new IpcError({ code: 'NotFound', message: 'file not found' }))
        const errors: unknown[] = []

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: false, target: null }, { onError: (error) => errors.push(error) })
        await waitFor(() => expect(errors.length).toBe(1))

        expect(isIndexInvalidated(queryClient, PROJECT_ID)).toBe(true)
        expect(isIndexInvalidated(queryClient, OTHER_PROJECT_ID)).toBe(false)
    })

    test('NotFound 가 아닌 실패는 인덱스를 건드리지 않는다 (Forbidden·Io 는 목록의 신선도와 무관)', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        openTabImpl.current = () => Promise.reject(new IpcError({ code: 'Forbidden', message: 'outside project root' }))
        const errors: unknown[] = []

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: false, target: null }, { onError: (error) => errors.push(error) })
        await waitFor(() => expect(errors.length).toBe(1))

        expect(isIndexInvalidated(queryClient, PROJECT_ID)).toBe(false)
        expect(isIndexInvalidated(queryClient, OTHER_PROJECT_ID)).toBe(false)
    })

    test('IpcError 가 아닌 평범한 예외도 onError 로만 전달하고 인덱스는 그대로 둔다', async () => {
        const { useOpenFileTab } = await importLayoutQuery()
        const queryClient = await setupIndexes()
        const failure = new Error('unexpected')
        openTabImpl.current = () => Promise.reject(failure)
        const errors: unknown[] = []

        const { result } = renderHookWithProviders(() => useOpenFileTab(), { queryClient })
        result.current({ projectId: PROJECT_ID, path: FILE_PATH, preview: false, target: null }, { onError: (error) => errors.push(error) })
        await waitFor(() => expect(errors.length).toBe(1))

        expect(errors[0]).toBe(failure)
        expect(isIndexInvalidated(queryClient, PROJECT_ID)).toBe(false)
    })
})
