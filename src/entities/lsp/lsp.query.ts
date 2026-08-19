import { useSyncExternalStore } from 'react'
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
 * eviction also uses). `widgets/settings-view/settings-view.tsx` reads it directly — no query cache
 * involved.
 */
const lspInstallProgressStore = createExternalStoreBridge<Record<string, LspInstallProgress>>({})

export const useLspInstallProgress = () => useSyncExternalStore(lspInstallProgressStore.subscribe, lspInstallProgressStore.getSnapshot)

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
 * R7#1: `QUERY_KEY.LSP.SESSIONS` (`lsp_sessions` polling — `LspSessionInfo`'s own `generation` field
 * is the poll-based fallback for a renderer that missed the push event entirely) previously had
 * zero invalidation call sites anywhere in the app — nothing ever told a mounted `useQuery` to
 * refetch it, so a session's `status`/`generation` only ever updated for a consumer that happened
 * to remount. Every `lsp:session-status-changed` push (spawn/stop/crash/restart/reinitialize-confirm
 * — `lsp-session-registry.ts`'s module-level listener drives the actual reinitialize handshake for
 * the crash case; this hook only owns the query cache) invalidates broadly via `QUERY_KEY.LSP.ALL`
 * rather than a specific `QUERY_KEY.LSP.SESSIONS(projectId)` — the event payload carries no
 * `projectId` to scope to, and adding a project-scoped lookup here would mean either a new
 * `QUERY_KEY.LSP.SESSIONS_ALL` prefix entry (`shared/constants/query-key.ts` is a concurrently
 * edited file outside this change's ownership) or reaching into `lsp-session-registry.ts`'s
 * internal session map from this entities-layer hook (that module pulls in real monaco worker
 * bundles unrelated code shouldn't have to load — the same reason `lsp-session-flush-registry.ts`
 * exists as an indirection). `LSP.SERVERS` incidentally invalidating too is an acceptable,
 * infrequent-event-driven cost, not a correctness issue.
 *
 * Mounted in `app/providers/ipc-sync-provider.tsx` (Phase D integration — `IpcSyncProvider` is
 * always-active in both the main and auxiliary window provider trees, the same
 * always-mounted-provider requirement `frontend.md`'s T1-A pattern calls for).
 */
export const useLspSessionsQueryInvalidationSync = () => {
    const queryClient = useQueryClient()
    useTauriEvent(events.lspSessionStatusChanged, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.LSP.ALL })
    })
}
