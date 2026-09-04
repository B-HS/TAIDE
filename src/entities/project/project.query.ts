import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import type { ProjectDisplayPatch, ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import {
    activateProject,
    closeProject,
    getActiveProjectId,
    getProject,
    listProjects,
    listRecentProjects,
    openProject,
    reorderProjects,
    setProjectDisplay,
} from '@entities/project/project.ipc'

export const projectListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.PROJECT.LIST, queryFn: listProjects })

/**
 * The Welcome screen's "recent projects" source — every persisted project record on this desktop
 * (not only the currently-open ones `projectListQueryOptions` returns), most-recently-opened
 * first. Covered by `QUERY_KEY.PROJECT.ALL`'s prefix, so `useOpenProject`/`useActivateProject`'s
 * existing `invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })` already refreshes this list —
 * no additional invalidation wiring needed.
 */
export const recentProjectsQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.PROJECT.RECENT, queryFn: listRecentProjects })

export const activeProjectQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.PROJECT.ACTIVE, queryFn: getActiveProjectId })

export const projectQueryOptions = (projectId: ProjectId) =>
    queryOptions({ queryKey: QUERY_KEY.PROJECT.DETAIL(projectId), queryFn: () => getProject(projectId) })

export const useOpenProject = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: openProject,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL }),
    })
}

/**
 * Native "choose a folder, then open it as a project" flow — the sidebar's `+` button and the
 * Welcome screen's "Open Folder" button both need exactly this sequence, so it's centralized here
 * once a second call site appeared (common.md's "2 회 이상" rule) rather than each widget
 * re-implementing the dialog + mutation + toast trio.
 */
export const useOpenFolderDialog = () => {
    const { mutate: openProjectMutate } = useOpenProject()
    return () => {
        void (async () => {
            const selected = await open({ directory: true, multiple: false })
            if (typeof selected !== 'string') return
            openProjectMutate(selected, { onError: (error) => toast.error(describeIpcError(error)) })
        })()
    }
}

export const useCloseProject = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: closeProject,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL }),
    })
}

export const useActivateProject = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: activateProject,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL }),
    })
}

export const useReorderProjects = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: reorderProjects,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.LIST }),
    })
}

/**
 * `project_set_display` also emits `ProjectListChanged`, which `ipc-sync-provider` already turns
 * into the same invalidation for every window — this `onSuccess` exists for immediacy in the window
 * that made the change, so the sidebar button repaints without waiting for the event round trip
 * (the same reason `useReorderProjects` invalidates rather than relying on the event alone).
 */
export const useSetProjectDisplay = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: ({ projectId, patch }: { projectId: ProjectId; patch: ProjectDisplayPatch }) => setProjectDisplay(projectId, patch),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.LIST }),
    })
}
