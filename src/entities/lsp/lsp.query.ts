import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { LspInstallProgress, LspServerId, ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { cancelLspInstall, detectLspServers, installLspServer, listLspSessions } from '@entities/lsp/lsp.ipc'

export const lspServersQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.LSP.SERVERS, queryFn: detectLspServers, staleTime: Infinity })

export const lspSessionsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.LSP.SESSIONS(projectId ?? ''),
        queryFn: () => listLspSessions(projectId ?? ''),
        enabled: !!projectId,
    })

export const lspInstallProgressQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.LSP.INSTALL_PROGRESS,
        queryFn: () => Promise.resolve<Record<string, LspInstallProgress>>({}),
        staleTime: Infinity,
    })

export const useLspInstallProgressSync = () => {
    const queryClient = useQueryClient()
    useTauriEvent(events.lspInstallProgress, ({ payload }) => {
        queryClient.setQueryData(QUERY_KEY.LSP.INSTALL_PROGRESS, (previous: Record<string, LspInstallProgress> = {}) => ({
            ...previous,
            [payload.serverId]: payload,
        }))
    })
}

export const useInstallLspServer = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (serverId: LspServerId) => installLspServer(serverId),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.LSP.SERVERS }),
        onError: (_error, serverId) =>
            queryClient.setQueryData(QUERY_KEY.LSP.INSTALL_PROGRESS, (previous: Record<string, LspInstallProgress> = {}) => {
                const current = previous[serverId]
                if (!current || current.phase === 'failed' || current.phase === 'done') return previous
                return Object.fromEntries(Object.entries(previous).filter(([id]) => id !== serverId))
            }),
    })
}

export const useCancelLspInstall = () => useMutation({ mutationFn: (serverId: LspServerId) => cancelLspInstall(serverId) })
