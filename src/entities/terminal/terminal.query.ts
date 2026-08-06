import { queryOptions } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { listShellProfiles, listTerminalSessions } from '@entities/terminal/terminal.ipc'

export const shellProfilesQueryOptions = () =>
    queryOptions({ queryKey: QUERY_KEY.TERMINAL.PROFILES, queryFn: listShellProfiles, staleTime: Infinity })

export const terminalSessionsQueryOptions = (projectId: ProjectId) =>
    queryOptions({ queryKey: QUERY_KEY.TERMINAL.SESSIONS(projectId), queryFn: () => listTerminalSessions(projectId), staleTime: Infinity })
