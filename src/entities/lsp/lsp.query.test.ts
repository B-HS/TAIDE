import { describe, expect, mock, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'

/**
 * `@entities/lsp/lsp.ipc` re-exports Tauri command bindings (`@shared/api/bindings`) that this bun
 * test environment cannot load — `lsp-session-registry.test.ts` mocks the same module for the same
 * reason. `mock.module` is process-global (not scoped to one test file) and last-registration-wins,
 * so whichever of these two mocks happens to load last would otherwise silently replace the other
 * for the whole test run; this fake covers the module's *entire* export surface (not just what this
 * file's own tests exercise) so it stays a safe stand-in for `lsp-session-registry.test.ts`'s own
 * dynamic import too, regardless of file load order. Nothing here needs to do anything real — only
 * `detectLspServers`'s queryFn wiring in `lsp.query.ts` is ever actually invoked by this file's tests.
 */
mock.module('@entities/lsp/lsp.ipc', () => ({
    spawnLspSession: () => Promise.resolve('fake-session'),
    sendLspMessage: () => Promise.resolve(),
    stopLspSession: () => Promise.resolve(),
    restartLspSession: () => Promise.resolve(),
    confirmLspReinitialize: () => Promise.resolve(),
    listLspSessions: () => Promise.resolve([]),
    detectLspServers: () => Promise.resolve([]),
    resolveLspRoot: () => Promise.resolve(null),
    installLspServer: () => Promise.resolve(),
    cancelLspInstall: () => Promise.resolve(),
}))

const importLspQuery = () => import('@entities/lsp/lsp.query')

describe('isLspServersQueryKey', () => {
    test('QUERY_KEY.LSP.SERVERS 와 정확히 일치하는 키만 참이다', async () => {
        const { isLspServersQueryKey } = await importLspQuery()
        expect(isLspServersQueryKey(QUERY_KEY.LSP.SERVERS)).toBe(true)
        expect(isLspServersQueryKey(QUERY_KEY.LSP.ALL)).toBe(false)
        expect(isLspServersQueryKey(QUERY_KEY.LSP.SESSIONS('project-1'))).toBe(false)
    })
})

describe('invalidateLspSessionsQueryKeys — lsp:session-status-changed 무효화 스코프 (contract T1-D F7#4/R7#1)', () => {
    test('LSP.SESSIONS(projectId) 는 무효화하지만 staleTime:Infinity 인 LSP.SERVERS 는 건드리지 않는다', async () => {
        const { invalidateLspSessionsQueryKeys } = await importLspQuery()
        const queryClient = new QueryClient()
        await queryClient.fetchQuery({ queryKey: QUERY_KEY.LSP.SERVERS, queryFn: () => Promise.resolve([]), staleTime: Infinity })
        await queryClient.fetchQuery({ queryKey: QUERY_KEY.LSP.SESSIONS('project-1'), queryFn: () => Promise.resolve([]) })
        await queryClient.fetchQuery({ queryKey: QUERY_KEY.LSP.SESSIONS('project-2'), queryFn: () => Promise.resolve([]) })

        await invalidateLspSessionsQueryKeys(queryClient)

        expect(queryClient.getQueryState(QUERY_KEY.LSP.SERVERS)?.isInvalidated).toBe(false)
        expect(queryClient.getQueryState(QUERY_KEY.LSP.SESSIONS('project-1'))?.isInvalidated).toBe(true)
        expect(queryClient.getQueryState(QUERY_KEY.LSP.SESSIONS('project-2'))?.isInvalidated).toBe(true)
    })
})
