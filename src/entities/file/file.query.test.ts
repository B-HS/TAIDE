import { createElement } from 'react'
import { renderToString } from 'react-dom/server'
import { describe, expect, mock, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'

/**
 * `file.query.ts` reaches monaco (through `tab-path-change.ts` → `model-registry`) and two `.ipc`
 * modules at import time, so the same three seams `tab-path-change.test.ts` stubs are stubbed here
 * before the module is pulled in through a *dynamic* `import()`. `mock.module` is process-global and
 * last-registration-wins (`lsp.query.test.ts` documents the hazard), so both `.ipc` fakes cover
 * their module's *entire* export surface — a partial fake left behind here would turn into a
 * "named export not found" SyntaxError in whichever later test file imports the real consumer.
 *
 * The mutations' own IPC calls resolve to nothing; `applyTabPathChange` rejects the way a real
 * `invoke` does under `bun:test` (no `window.__TAURI_INTERNALS__`), which doubles as coverage that
 * the index refresh still runs when the tab-follow step fails (its rejection is swallowed by design).
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

mock.module('@entities/file/file.ipc', () => ({
    openFile: () => Promise.resolve(null),
    saveFile: () => Promise.resolve(null),
    createEntry: () => Promise.resolve(null),
    renameEntry: () => Promise.resolve(null),
    deleteEntry: () => Promise.resolve(null),
    copyEntry: () => Promise.resolve(null),
    mirrorDirty: () => Promise.resolve(null),
    listMirrors: () => Promise.resolve([]),
    clearMirror: () => Promise.resolve(null),
    pruneMirrors: () => Promise.resolve(null),
    mirrorUntitled: () => Promise.resolve(null),
    listUntitledMirrors: () => Promise.resolve([]),
    clearUntitledMirror: () => Promise.resolve(null),
    pruneUntitledMirrors: () => Promise.resolve(null),
    flushMirrorsComplete: () => Promise.resolve(null),
}))

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

mock.module('@entities/layout/layout.ipc', () => ({
    getLayout: rejectLikeUnavailableIpc,
    openTab: rejectLikeUnavailableIpc,
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

const importFileQuery = () => import('@entities/file/file.query')

const PROJECT_ID = 'project-1'
const OTHER_PROJECT_ID = 'project-2'
const FILE_PATH = '/project/src/a.ts'
const RENAMED_PATH = '/project/src/b.ts'

/**
 * Runs a query hook once through a server render so its `useMutation` result can be driven
 * without a DOM: `QueryClientProvider` supplies the client, the mutation observer is created in the
 * hook's own `useState` initializer with its full options (`mutationFn` + `onSuccess`), and
 * `mutateAsync` executes against that observer exactly as it would after a client render.
 */
const renderQueryHook = <TArgs extends unknown[], TResult>(queryClient: QueryClient, useHook: (...args: TArgs) => TResult, ...args: TArgs) => {
    let captured: TResult | undefined
    const Probe = () => {
        captured = useHook(...args)
        return null
    }
    renderToString(createElement(QueryClientProvider, { client: queryClient }, createElement(Probe)))
    if (captured === undefined) throw new Error('hook did not render')
    return captured
}

/** Puts a quick-open index for `projectId` in the cache with no observer attached and returns its fetch counter. */
const seedProjectFileIndex = async (queryClient: QueryClient, projectId: string) => {
    const fetches = { count: 0 }
    await queryClient.fetchQuery({
        queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId),
        queryFn: () => {
            fetches.count += 1
            return Promise.resolve([])
        },
    })
    return fetches
}

const waitForIdle = async (queryClient: QueryClient) => {
    while (queryClient.isFetching() > 0) await new Promise((resolve) => setTimeout(resolve, 0))
}

const setupIndexes = async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    const own = await seedProjectFileIndex(queryClient, PROJECT_ID)
    const other = await seedProjectFileIndex(queryClient, OTHER_PROJECT_ID)
    return { queryClient, own, other }
}

describe('앱 내부 fs 뮤테이션의 퀵오픈 인덱스 무효화 (contract 2026-09-04 batch3 §A.2 item 5)', () => {
    test('useCreateEntry 성공 시 해당 프로젝트의 SEARCH.PROJECT_FILES 를 무효화하고 관찰자 없는 쿼리도 재조회한다 (refetchType: all)', async () => {
        const { useCreateEntry } = await importFileQuery()
        const { queryClient, own, other } = await setupIndexes()

        const mutation = renderQueryHook(queryClient, useCreateEntry, PROJECT_ID)
        await mutation.mutateAsync({ path: FILE_PATH, isDir: false })
        await waitForIdle(queryClient)

        expect(own.count).toBe(2)
        expect(other.count).toBe(1)
        expect(queryClient.getQueryState(QUERY_KEY.SEARCH.PROJECT_FILES(OTHER_PROJECT_ID))?.isInvalidated).toBe(false)
    })

    test('useRenameEntry 성공 시 탭 이동(applyTabPathChange)이 실패해도 인덱스를 무효화·재조회한다', async () => {
        const { useRenameEntry } = await importFileQuery()
        const { queryClient, own, other } = await setupIndexes()

        const mutation = renderQueryHook(queryClient, useRenameEntry, PROJECT_ID)
        await mutation.mutateAsync({ from: FILE_PATH, to: RENAMED_PATH })
        await waitForIdle(queryClient)

        expect(own.count).toBe(2)
        expect(other.count).toBe(1)
    })

    test('useCopyEntry 성공 시 인덱스를 무효화·재조회한다', async () => {
        const { useCopyEntry } = await importFileQuery()
        const { queryClient, own, other } = await setupIndexes()

        const mutation = renderQueryHook(queryClient, useCopyEntry, PROJECT_ID)
        await mutation.mutateAsync({ from: FILE_PATH, to: RENAMED_PATH })
        await waitForIdle(queryClient)

        expect(own.count).toBe(2)
        expect(other.count).toBe(1)
    })

    test('useDeleteEntry 성공 시 탭 닫기(applyTabPathChange)가 실패해도 인덱스를 무효화·재조회한다', async () => {
        const { useDeleteEntry } = await importFileQuery()
        const { queryClient, own, other } = await setupIndexes()

        const mutation = renderQueryHook(queryClient, useDeleteEntry, PROJECT_ID)
        await mutation.mutateAsync(FILE_PATH)
        await waitForIdle(queryClient)

        expect(own.count).toBe(2)
        expect(other.count).toBe(1)
    })

    test('useSaveFile 은 projectId 를 받아도 인덱스를 건드리지 않는다 (저장은 이미 인덱스에 있는 경로 — 자동 저장마다 전체 walk 방지)', async () => {
        const { useSaveFile } = await importFileQuery()
        const { queryClient, own, other } = await setupIndexes()

        const mutation = renderQueryHook(queryClient, useSaveFile, PROJECT_ID)
        await mutation.mutateAsync({ path: FILE_PATH, content: 'saved' })
        await waitForIdle(queryClient)

        expect(own.count).toBe(1)
        expect(other.count).toBe(1)
        expect(queryClient.getQueryState(QUERY_KEY.SEARCH.PROJECT_FILES(PROJECT_ID))?.isInvalidated).toBe(false)
    })
})

/**
 * §C.2-4 M3 reclaims `FILE.CONTENT`/`FILE.RAW` when a path's last tab closes. d-43
 * (`2026-08-27-d43-save-stale-sync-clobber-contract.md`) depends on the *opposite* guarantee for a
 * path that is still open: `useSaveFile`'s `onSuccess` patches the cached entry synchronously so a
 * pane's dirty→false settle re-adopts the just-saved text instead of the pre-save text. This pins
 * that the reclaim never reaches the save path — saving must leave a patched entry behind, not a
 * hole that the next open would have to re-fetch through the very window d-43 closed.
 */
describe('저장 경로는 캐시 회수와 무관하다 (d-43 클로버 계약 유지)', () => {
    test('useSaveFile 성공 후 FILE.CONTENT 엔트리가 남아 있고 방금 저장한 내용으로 패치된다', async () => {
        const { useSaveFile } = await importFileQuery()
        const { queryClient } = await setupIndexes()
        queryClient.setQueryData(QUERY_KEY.FILE.CONTENT(FILE_PATH), { path: FILE_PATH, content: 'before', languageId: 'typescript' })

        const mutation = renderQueryHook(queryClient, useSaveFile, PROJECT_ID)
        await mutation.mutateAsync({ path: FILE_PATH, content: 'saved' })

        expect(queryClient.getQueryData(QUERY_KEY.FILE.CONTENT(FILE_PATH))).toMatchObject({ path: FILE_PATH, content: 'saved' })
    })
})
