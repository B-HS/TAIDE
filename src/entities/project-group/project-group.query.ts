import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ProjectGroupId, ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import {
    createProjectGroup,
    deleteProjectGroup,
    listProjectGroups,
    openProjectGroup,
    renameProjectGroup,
    reorderProjectGroups,
    setProjectGroupCollapsed,
    setProjectGroupColor,
    setProjectGroupMembers,
} from '@entities/project-group/project-group.ipc'

/**
 * The sidebar's groups. Read once per window and refreshed from `project:groups-changed`
 * (`ipc-sync-provider.tsx`) — the event fires only at a transition, which is why the list command
 * exists at all (`session_get_shell_state` has the same shape for the same reason).
 */
export const projectGroupListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.PROJECT_GROUP.LIST, queryFn: listProjectGroups })

/**
 * Every group write answers with `ProjectGroupsChanged`, which `ipc-sync-provider` turns into this
 * same invalidation for every window. This local one exists for immediacy in the window that made
 * the change, so a renamed header or a collapsed section repaints without waiting for the event
 * round trip — the arrangement `useSetProjectDisplay` already uses.
 */
const useProjectGroupMutation = <TVariables, TData>(mutationFn: (variables: TVariables) => Promise<TData>) => {
    const queryClient = useQueryClient()
    return useMutation({ mutationFn, onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT_GROUP.ALL }) })
}

export const useCreateProjectGroup = () =>
    useProjectGroupMutation(({ name, color, members }: { name: string; color: string | null; members: ProjectId[] | null }) =>
        createProjectGroup(name, color, members),
    )

export const useRenameProjectGroup = () =>
    useProjectGroupMutation(({ groupId, name }: { groupId: ProjectGroupId; name: string }) => renameProjectGroup(groupId, name))

export const useSetProjectGroupColor = () =>
    useProjectGroupMutation(({ groupId, color }: { groupId: ProjectGroupId; color: string | null }) => setProjectGroupColor(groupId, color))

export const useSetProjectGroupCollapsed = () =>
    useProjectGroupMutation(({ groupId, collapsed }: { groupId: ProjectGroupId; collapsed: boolean }) => setProjectGroupCollapsed(groupId, collapsed))

export const useSetProjectGroupMembers = () =>
    useProjectGroupMutation(({ groupId, members }: { groupId: ProjectGroupId; members: ProjectId[] }) => setProjectGroupMembers(groupId, members))

export const useDeleteProjectGroup = () => useProjectGroupMutation(deleteProjectGroup)

export const useReorderProjectGroups = () => useProjectGroupMutation(reorderProjectGroups)

/**
 * Opening a group changes which projects are open, not the groups themselves — Rust says so by
 * emitting `ProjectListChanged` per member and no `ProjectGroupsChanged` at all — so this is the one
 * group mutation that invalidates the project list instead of the group list.
 */
export const useOpenProjectGroup = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: openProjectGroup,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL }),
    })
}
