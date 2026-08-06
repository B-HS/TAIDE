import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ProjectId, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import {
    activateTab,
    closeTab,
    focusPane,
    getLayout,
    moveTab,
    openTab,
    pinTab,
    reopenClosedTab,
    resizePane,
    setTabDirty,
    setTerminalSession,
    splitPane,
} from '@entities/layout/layout.ipc'

export const layoutQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.LAYOUT.DETAIL(projectId ?? ''),
        queryFn: () => getLayout(projectId ?? ''),
        enabled: !!projectId,
    })

const useLayoutMutation = <TVariables>(projectId: ProjectId | null, mutationFn: (variables: TVariables) => Promise<ProjectLayout>) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn,
        onSuccess: (layout) => queryClient.setQueryData(QUERY_KEY.LAYOUT.DETAIL(projectId ?? ''), layout),
    })
}

export const useOpenTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, openTab)

export const useCloseTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, closeTab)

export const useActivateTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, activateTab)

export const useMoveTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, moveTab)

export const useSplitPane = (projectId: ProjectId | null) => useLayoutMutation(projectId, splitPane)

export const useResizePane = (projectId: ProjectId | null) => useLayoutMutation(projectId, resizePane)

export const useFocusPane = (projectId: ProjectId | null) => useLayoutMutation(projectId, focusPane)

export const usePinTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, pinTab)

export const useSetTabDirty = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTabDirty)

export const useSetTerminalSession = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTerminalSession)

export const useReopenClosedTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, reopenClosedTab)
