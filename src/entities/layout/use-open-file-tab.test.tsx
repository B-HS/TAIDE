import { describe, expect, mock, test } from 'bun:test'
import type { PaneNode, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { IpcError } from '@shared/api/unwrap-result'
import { createTestQueryClient, renderHookWithProviders, waitFor } from '@shared/testing/render'

/**
 * `useOpenFileTab` is the single entry point every file open funnels through (batch 3 contract
 * §A.2 item 4), and the branch worth locking is the quick-open index repair: a `NotFound` means the
 * palette's `SEARCH.PROJECT_FILES` snapshot handed out a path that is no longer on disk, so that
 * listing — and only that project's — is invalidated, while any other failure says nothing about
 * the index and must leave it alone.
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

const buildLayout = (revision: number): ProjectLayout => ({ version: 2, root: EMPTY_LEAF, focusedPane: 'leaf-1', revision })

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

    test('제목을 명시하면 파일명 대신 그 제목을 쓴다', async () => {
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
