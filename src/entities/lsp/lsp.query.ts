import { useSyncExternalStore } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { LspInstallProgress, LspServerId, ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { createExternalStoreBridge } from '@shared/lib/external-store-bridge'
import { cancelLspInstall, detectLspServers, installLspServer, listLspSessions } from '@entities/lsp/lsp.ipc'

export const lspServersQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.LSP.SERVERS, queryFn: detectLspServers, staleTime: Infinity })

export const lspSessionsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.LSP.SESSIONS(projectId ?? ''),
        queryFn: () => listLspSessions(projectId ?? ''),
        enabled: !!projectId,
    })

/**
 * F1#10 (decision #2, `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md` §1.0): install
 * progress is client-only, event-pushed state with no server-fetchable counterpart — putting it in
 * the query cache (a `queryFn` that never actually fetched anything, fed entirely by `setQueryData`)
 * was a category error. `useLspInstallProgress` is the correctly-layered replacement
 * (`external-store-bridge.ts`, the same primitive `shared/lib` factory F1#11's theme-preview
 * eviction also uses). `widgets/settings-view/settings-lsp-section.tsx` reads it directly — no
 * query cache involved.
 */
const lspInstallProgressStore = createExternalStoreBridge<Record<string, LspInstallProgress>>({})

export const useLspInstallProgress = () => useSyncExternalStore(lspInstallProgressStore.subscribe, lspInstallProgressStore.getSnapshot)

/**
 * T2-B Phase E (`docs/acknowledge/2026-08-21-t2b-settings-view-contract.md` §4): called from
 * `settings-view.tsx` (the container), not from `settings-lsp-section.tsx` — the container is the
 * one component in the settings screen that stays mounted across the ThemeEditor/SnippetEditor
 * full-screen swap (`themeEditorState`/`isSnippetEditorOpen` early returns), so subscribing there
 * keeps this Tauri listener alive for the swap's duration instead of dropping install-progress
 * events whenever the LSP section itself is unmounted.
 */
export const useLspInstallProgressSync = () => {
    useTauriEvent(events.lspInstallProgress, ({ payload }) => {
        lspInstallProgressStore.setValue({ ...lspInstallProgressStore.getSnapshot(), [payload.serverId]: payload })
    })
}

export const useInstallLspServer = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (serverId: LspServerId) => installLspServer(serverId),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.LSP.SERVERS }),
        onError: (_error, serverId) => {
            const dropIfSettled = (previous: Record<string, LspInstallProgress>) => {
                const current = previous[serverId]
                if (!current || current.phase === 'failed' || current.phase === 'done') return previous
                return Object.fromEntries(Object.entries(previous).filter(([id]) => id !== serverId))
            }
            lspInstallProgressStore.setValue(dropIfSettled(lspInstallProgressStore.getSnapshot()))
        },
    })
}

export const useCancelLspInstall = () => useMutation({ mutationFn: (serverId: LspServerId) => cancelLspInstall(serverId) })

/**
 * A `lsp:session-status-changed` query key belongs to `QUERY_KEY.LSP.SERVERS` (as opposed to every
 * other `QUERY_KEY.LSP.*` branch, i.e. `SESSIONS(projectId)`) — used below to invalidate every
 * project's sessions query without also nuking `lspServersQueryOptions`' `staleTime: Infinity` PATH
 * scan (`lsp_detect_servers`), which nothing about a session spawning/crashing/stopping should ever
 * re-trigger.
 */
export const isLspServersQueryKey = (queryKey: readonly unknown[]) =>
    queryKey.length === QUERY_KEY.LSP.SERVERS.length && queryKey.every((segment, index) => segment === QUERY_KEY.LSP.SERVERS[index])

/**
 * Split out from {@link useLspSessionsQueryInvalidationSync} so the invalidation-scope decision is
 * a plain, directly testable function of a `QueryClient` rather than only reachable by rendering the
 * hook (this module has no existing hook-render test harness to reach for).
 */
export const invalidateLspSessionsQueryKeys = (queryClient: QueryClient) =>
    queryClient.invalidateQueries({ queryKey: QUERY_KEY.LSP.ALL, predicate: (query) => !isLspServersQueryKey(query.queryKey) })

/**
 * R7#1: `QUERY_KEY.LSP.SESSIONS` (`lsp_sessions` polling — `LspSessionInfo`'s own `generation` field
 * is the poll-based fallback for a renderer that missed the push event entirely) previously had
 * zero invalidation call sites anywhere in the app — nothing ever told a mounted `useQuery` to
 * refetch it, so a session's `status`/`generation` only ever updated for a consumer that happened
 * to remount. Every `lsp:session-status-changed` push (spawn/stop/crash/restart/reinitialize-confirm
 * — `lsp-session-registry.ts`'s module-level listener drives the actual reinitialize handshake for
 * the crash case; this hook only owns the query cache) invalidates every `QUERY_KEY.LSP.ALL`-prefixed
 * query *except* `SERVERS` (`isLspServersQueryKey` above) — the event payload carries no `projectId`
 * to scope down to a single `QUERY_KEY.LSP.SESSIONS(projectId)`, but excluding `SERVERS` by its own
 * already-exported key keeps `lspServersQueryOptions`' one-scan-per-boot contract intact without
 * reaching into `lsp-session-registry.ts`'s internal session map from this entities-layer hook (that
 * module pulls in real monaco worker bundles unrelated code shouldn't have to load — the same reason
 * `lsp-session-flush-registry.ts` exists as an indirection).
 *
 * Mounted in `app/providers/ipc-sync-provider.tsx` (Phase D integration — `IpcSyncProvider` is
 * always-active in both the main and auxiliary window provider trees, the same
 * always-mounted-provider requirement `frontend.md`'s T1-A pattern calls for).
 */
export const useLspSessionsQueryInvalidationSync = () => {
    const queryClient = useQueryClient()
    useTauriEvent(events.lspSessionStatusChanged, () => {
        void invalidateLspSessionsQueryKeys(queryClient)
    })
}
