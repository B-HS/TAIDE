import { createElement } from 'react'
import { renderToString } from 'react-dom/server'
import { describe, expect, mock, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'

/**
 * `project.query.ts` reaches `project.ipc.ts` (Tauri command bindings) at import time, so the
 * module is stubbed before `project.query` is pulled in through a *dynamic* `import()`, the same
 * way `file.query.test.ts` does. `mock.module` is process-global and last-registration-wins
 * (`lsp.query.test.ts` documents the hazard), so this fake covers the module's *entire* export
 * surface — `use-editor-lsp-integration.test.ts` registers its own fake of the same module without
 * `setProjectDisplay`, and whichever file runs later must not leave a partial surface behind.
 */
const capturedSetProjectDisplayCalls: { projectId: string; patch: unknown }[] = []

mock.module('@entities/project/project.ipc', () => ({
    listProjects: () => Promise.resolve([]),
    listRecentProjects: () => Promise.resolve([]),
    getProject: () => Promise.resolve(null),
    getActiveProjectId: () => Promise.resolve(null),
    openProject: () => Promise.resolve(null),
    closeProject: () => Promise.resolve(undefined),
    activateProject: () => Promise.resolve(undefined),
    reorderProjects: () => Promise.resolve(undefined),
    setProjectDisplay: (projectId: string, patch: unknown) => {
        capturedSetProjectDisplayCalls.push({ projectId, patch })
        return Promise.resolve(undefined)
    },
}))

const importProjectQuery = () => import('@entities/project/project.query')

const PROJECT_ID = 'project-1'

const renderQueryHook = <TResult>(queryClient: QueryClient, useHook: () => TResult) => {
    let captured: TResult | undefined
    const Probe = () => {
        captured = useHook()
        return null
    }
    renderToString(createElement(QueryClientProvider, { client: queryClient }, createElement(Probe)))
    if (captured === undefined) throw new Error('hook did not render')
    return captured
}

/** Puts a query for `queryKey` in the cache with no observer attached and returns its fetch counter. */
const seedQuery = async (queryClient: QueryClient, queryKey: readonly unknown[]) => {
    const fetches = { count: 0 }
    await queryClient.fetchQuery({
        queryKey,
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

const setupProjectQueries = async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    const list = await seedQuery(queryClient, QUERY_KEY.PROJECT.LIST)
    const recent = await seedQuery(queryClient, QUERY_KEY.PROJECT.RECENT)
    const active = await seedQuery(queryClient, QUERY_KEY.PROJECT.ACTIVE)
    return { queryClient, list, recent, active }
}

describe('useSetProjectDisplay 의 목록 무효화 (contract 2026-09-04 batch4 §D.2-2)', () => {
    test('성공 시 PROJECT.LIST 만 무효화하고 RECENT·ACTIVE 는 건드리지 않는다 (기본 refetchType: active — 관찰자 없는 쿼리는 재조회하지 않는다)', async () => {
        const { useSetProjectDisplay } = await importProjectQuery()
        const { queryClient, list, recent, active } = await setupProjectQueries()

        const mutation = renderQueryHook(queryClient, useSetProjectDisplay)
        await mutation.mutateAsync({ projectId: PROJECT_ID, patch: { icon: 'rocket', label: null, color: null } })
        await waitForIdle(queryClient)

        expect(queryClient.getQueryState(QUERY_KEY.PROJECT.LIST)?.isInvalidated).toBe(true)
        expect(queryClient.getQueryState(QUERY_KEY.PROJECT.RECENT)?.isInvalidated).toBe(false)
        expect(queryClient.getQueryState(QUERY_KEY.PROJECT.ACTIVE)?.isInvalidated).toBe(false)
        expect(list.count).toBe(1)
        expect(recent.count).toBe(1)
        expect(active.count).toBe(1)
    })

    test('projectId 와 patch 를 그대로 IPC 에 넘긴다', async () => {
        const { useSetProjectDisplay } = await importProjectQuery()
        const { queryClient } = await setupProjectQueries()
        const patch = { icon: '', label: 'TA', color: 'lane3' }

        const mutation = renderQueryHook(queryClient, useSetProjectDisplay)
        await mutation.mutateAsync({ projectId: PROJECT_ID, patch })

        expect(capturedSetProjectDisplayCalls.at(-1)).toEqual({ projectId: PROJECT_ID, patch })
    })
})
