import { queryOptions, useQueryClient } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { getCliInstallStatus, listProjectAgents } from '@entities/agent/agent.ipc'

export const projectAgentsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.AGENT.PROJECT(projectId ?? ''),
        queryFn: () => listProjectAgents(projectId ?? ''),
        enabled: !!projectId,
    })

export const cliInstallStatusQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.AGENT.CLI, queryFn: getCliInstallStatus })

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
