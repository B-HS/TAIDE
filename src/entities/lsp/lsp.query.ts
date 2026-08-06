import { queryOptions } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { detectLspServers, listLspSessions } from '@entities/lsp/lsp.ipc'

export const lspServersQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.LSP.SERVERS, queryFn: detectLspServers, staleTime: Infinity })

export const lspSessionsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.LSP.SESSIONS(projectId ?? ''),
        queryFn: () => listLspSessions(projectId ?? ''),
        enabled: !!projectId,
    })
