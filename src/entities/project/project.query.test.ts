import { createElement } from 'react'
import { renderToString } from 'react-dom/server'
import { describe, expect, mock, spyOn, test } from 'bun:test'
import * as sonner from 'sonner'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { EventCallback } from '@tauri-apps/api/event'
import type { ForgetRecentOutcome, ProjectRecentCleared } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, renderHookWithProviders } from '@shared/testing/render'

/**
 * `project.query.ts` reaches `project.ipc.ts` (Tauri command bindings) at import time, so the
 * module is stubbed before `project.query` is pulled in through a *dynamic* `import()`, the same
 * way `file.query.test.ts` does. `mock.module` is process-global and last-registration-wins
 * (`lsp.query.test.ts` documents the hazard), so this fake covers the module's *entire* export
 * surface — `use-editor-lsp-integration.test.ts` registers its own fake of the same module without
 * `setProjectDisplay`, and whichever file runs later must not leave a partial surface behind.
 */
const capturedSetProjectDisplayCalls: { projectId: string; patch: unknown }[] = []

/**
 * The real `sonner` namespace is snapshotted before the fake is registered so no export disappears
 * for whatever file runs next (`mock.module` is process-global — `docs/memory/test-conventions.md`
 * §3), the same shape `search-panel-container.test.tsx` uses.
 */
const realSonner = { ...sonner }
const infoMessages: string[] = []
const ignoreToast = () => undefined
const toastFake = Object.assign(ignoreToast, {
    ...realSonner.toast,
    info: (message: unknown) => {
        infoMessages.push(String(message))
    },
})

mock.module('sonner', () => ({ ...realSonner, toast: toastFake }))

const forgetRecentOutcome: { current: ForgetRecentOutcome } = { current: { removed: 0, skippedWithDrafts: 0, groupsChanged: false } }

mock.module('@entities/project/project.ipc', () => ({
    listProjects: () => Promise.resolve([]),
    listRecentProjects: () => Promise.resolve([]),
    getProject: () => Promise.resolve(null),
    getActiveProjectId: () => Promise.resolve(null),
    openProject: () => Promise.resolve(null),
    closeProject: () => Promise.resolve(undefined),
    activateProject: () => Promise.resolve(undefined),
    reorderProjects: () => Promise.resolve(undefined),
    forgetRecentProjects: () => Promise.resolve(forgetRecentOutcome.current),
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

const FAKE_EVENT_ID = 1

/** Delivers one `project:recent-cleared` to a freshly mounted {@link useRecentProjectsClearedNotice}, with the event's own `listen` spied so no backend is needed. */
const deliverRecentCleared = async (payload: ProjectRecentCleared) => {
    const delivery: { current: EventCallback<ProjectRecentCleared> | null } = { current: null }
    const listen = spyOn(events.projectRecentCleared, 'listen').mockImplementation((handler) => {
        delivery.current = handler
        return Promise.resolve(() => undefined)
    })

    const { useRecentProjectsClearedNotice } = await importProjectQuery()
    const rendered = renderHookWithProviders(useRecentProjectsClearedNotice)
    await act(async () => undefined)
    await act(async () => {
        delivery.current?.({ event: 'project:recent-cleared', id: FAKE_EVENT_ID, payload })
    })

    rendered.unmount()
    listen.mockRestore()
}

/**
 * d-67 #9 — `forget_recent_projects` now keeps a closed project whose hot-exit buffers still hold
 * unsaved drafts instead of deleting its directory (which used to destroy that work). The recent
 * list therefore comes back non-empty, and without the notice that reads as the command failing.
 *
 * The notice hangs off the `project:recent-cleared` event rather than off the mutation because the
 * only path that actually clears the list is the native `File > Clear Recent` menu, which Rust runs
 * without any window's mutation involved — on `useForgetRecentProjects.onSuccess` the notice was
 * unreachable in the running app (렌즈 major #9-FE).
 */
describe('useRecentProjectsClearedNotice 의 초안 보존 안내', () => {
    test('초안 때문에 건너뛴 프로젝트가 있으면 안내 토스트를 띄운다', async () => {
        const before = infoMessages.length

        await deliverRecentCleared({ removed: 2, skippedWithDrafts: 1 })

        expect(infoMessages.length).toBe(before + 1)
        expect(infoMessages.at(-1)).toContain('project.clearRecentKeptDrafts')
    })

    test('건너뛴 프로젝트가 없으면 아무것도 알리지 않는다', async () => {
        const before = infoMessages.length

        await deliverRecentCleared({ removed: 3, skippedWithDrafts: 0 })

        expect(infoMessages.length).toBe(before)
    })
})

describe('useForgetRecentProjects 의 목록 무효화', () => {
    test('성공하면 PROJECT.ALL 아래 목록을 전부 무효화한다 (지워진 항목이 남아 있으면 안 된다)', async () => {
        const { useForgetRecentProjects } = await importProjectQuery()
        const { queryClient } = await setupProjectQueries()
        forgetRecentOutcome.current = { removed: 1, skippedWithDrafts: 0, groupsChanged: false }

        const mutation = renderQueryHook(queryClient, useForgetRecentProjects)
        await mutation.mutateAsync(undefined)
        await waitForIdle(queryClient)

        expect(queryClient.getQueryState(QUERY_KEY.PROJECT.RECENT)?.isInvalidated).toBe(true)
        expect(queryClient.getQueryState(QUERY_KEY.PROJECT.LIST)?.isInvalidated).toBe(true)
    })
})
