import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { AgentHooksStatus, ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { getAgentHooksStatus, getCliInstallStatus, installAgentHooks, listProjectAgents, uninstallAgentHooks } from '@entities/agent/agent.ipc'

export const projectAgentsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.AGENT.PROJECT(projectId ?? ''),
        queryFn: () => listProjectAgents(projectId ?? ''),
        enabled: !!projectId,
    })

export const cliInstallStatusQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.AGENT.CLI, queryFn: getCliInstallStatus })

export const USER_LEVEL_AGENT_HOOKS_UNUSED_PROJECT_ID = ''

export const agentHooksStatusQueryOptions = (projectId: ProjectId, agentName: string) =>
    queryOptions({ queryKey: QUERY_KEY.AGENT.HOOKS(projectId, agentName), queryFn: () => getAgentHooksStatus(projectId, agentName) })

type AgentHooksMutationVariables = { projectId: ProjectId; agentName: string }

const useAgentHooksMutation = (mutationFn: (projectId: ProjectId, agentName: string) => Promise<AgentHooksStatus>) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: ({ projectId, agentName }: AgentHooksMutationVariables) => mutationFn(projectId, agentName),
        onSuccess: (status, { projectId, agentName }) => queryClient.setQueryData(QUERY_KEY.AGENT.HOOKS(projectId, agentName), status),
    })
}

export const useInstallAgentHooks = () => useAgentHooksMutation(installAgentHooks)

export const useUninstallAgentHooks = () => useAgentHooksMutation(uninstallAgentHooks)

/**
 * `agent:state-changed` 는 프로젝트의 완전한 최신 에이전트 목록을 push 로 전달한다.
 * 서버가 이미 확정한 전체 상태이므로 invalidateQueries 로 다시 물어보지 않고
 * setQueryData 로 캐시를 직접 채운다(query.md 의 "무효화 우선" 예외 — docs/acknowledge 기록).
 */
export const useAgentStateSync = () => {
    const queryClient = useQueryClient()
    useTauriEvent(events.agentStateChanged, ({ payload }) => {
        queryClient.setQueryData(QUERY_KEY.AGENT.PROJECT(payload.projectId), payload)
    })
}
