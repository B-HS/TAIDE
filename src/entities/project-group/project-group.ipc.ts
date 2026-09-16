import { commands } from '@shared/api/bindings'
import type { ProjectGroupId, ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listProjectGroups = () => unwrapResult(commands.projectGroupList())

export const createProjectGroup = (name: string, color: string | null, members: ProjectId[] | null) =>
    unwrapResult(commands.projectGroupCreate(name, color, members))

export const renameProjectGroup = (groupId: ProjectGroupId, name: string) => unwrapResult(commands.projectGroupRename(groupId, name))

export const setProjectGroupColor = (groupId: ProjectGroupId, color: string | null) => unwrapResult(commands.projectGroupSetColor(groupId, color))

export const setProjectGroupCollapsed = (groupId: ProjectGroupId, collapsed: boolean) =>
    unwrapResult(commands.projectGroupSetCollapsed(groupId, collapsed))

export const setProjectGroupMembers = (groupId: ProjectGroupId, members: ProjectId[]) =>
    unwrapResult(commands.projectGroupSetMembers(groupId, members))

export const deleteProjectGroup = (groupId: ProjectGroupId) => unwrapResult(commands.projectGroupDelete(groupId))

export const reorderProjectGroups = (ids: ProjectGroupId[]) => unwrapResult(commands.projectGroupReorder(ids))

export const openProjectGroup = (groupId: ProjectGroupId) => unwrapResult(commands.projectGroupOpen(groupId))
