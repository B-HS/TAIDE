import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ProjectId, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { findPaneTab } from '@shared/lib/pane-tree'
import { takeWaitMarkers } from '@entities/agent/agent-wait-marker-registry'
import { releaseWaitMarker } from '@entities/agent/agent.ipc'
import { clearMirror } from '@entities/file/file.ipc'
import { removePendingClaudeDiff } from '@entities/ide/claude-diff-registry'
import {
    activateTab,
    closeTab,
    convertUntitledTab,
    focusPane,
    getLayout,
    moveTab,
    moveTabToWindow,
    openTab,
    openUntitledTab,
    pinTab,
    reopenClosedTab,
    resizePane,
    setShellView,
    setTabDirty,
    setTabPreview,
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

/**
 * `useOpenTab` binds `projectId` at hook-call time, so it can't correctly cache a tab opened for a
 * *different* project than the one the calling widget was rendered for — the drag-and-drop-a-file
 * flow needs exactly that (the drop target may resolve to a project other than the currently active
 * one). This variant instead reads the target project from the mutation's own variables, matching
 * what `openTab`'s IPC call already receives (contract F4#4 — no more hand-rolled raw
 * `openTab()` + `setQueryData` in the widget).
 */
export const useOpenTabInProject = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: openTab,
        onSuccess: (layout, variables) => queryClient.setQueryData(QUERY_KEY.LAYOUT.DETAIL(variables.projectId), layout),
    })
}

export const useCloseTab = (projectId: ProjectId | null) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: closeTab,
        onSuccess: (layout, tabId) => {
            const previous = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(projectId ?? ''))
            const closedKind = previous ? findPaneTab(previous.root, tabId)?.kind : null
            if (closedKind?.kind === 'claudeDiff') removePendingClaudeDiff(closedKind.requestId)
            if (closedKind?.kind === 'file') {
                for (const marker of takeWaitMarkers(closedKind.path)) void releaseWaitMarker(marker)
                if (projectId) {
                    void clearMirror({ projectId, path: closedKind.path }).catch(() => undefined)
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
                }
            }
            queryClient.setQueryData(QUERY_KEY.LAYOUT.DETAIL(projectId ?? ''), layout)
        },
    })
}

export const useActivateTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, activateTab)

export const useMoveTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, moveTab)

export const useSplitPane = (projectId: ProjectId | null) => useLayoutMutation(projectId, splitPane)

export const useResizePane = (projectId: ProjectId | null) => useLayoutMutation(projectId, resizePane)

export const useFocusPane = (projectId: ProjectId | null) => useLayoutMutation(projectId, focusPane)

export const usePinTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, pinTab)

export const useSetTabPreview = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTabPreview)

export const useSetTabDirty = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTabDirty)

export const useSetTerminalSession = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTerminalSession)

export const useReopenClosedTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, reopenClosedTab)

export const useOpenUntitledTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, openUntitledTab)

export const useConvertUntitledTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, convertUntitledTab)

export const useMoveTabToWindow = (projectId: ProjectId | null) => useLayoutMutation(projectId, moveTabToWindow)

export const useSetShellView = (projectId: ProjectId | null) => useLayoutMutation(projectId, setShellView)
