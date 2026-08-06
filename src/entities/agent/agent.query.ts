import { queryOptions } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getCliInstallStatus, listProjectAgents } from '@entities/agent/agent.ipc'

export const projectAgentsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.AGENT.PROJECT(projectId ?? ''),
        queryFn: () => listProjectAgents(projectId ?? ''),
        enabled: !!projectId,
    })

export const cliInstallStatusQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.AGENT.CLI, queryFn: getCliInstallStatus })
