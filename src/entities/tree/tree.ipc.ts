import { commands } from '@shared/api/bindings'
import type { ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getTreeRows = (input: { projectId: ProjectId; offset: number; limit: number }) =>
    unwrapResult(commands.treeRows(input.projectId, input.offset, input.limit))

export const toggleTreeNode = (input: { projectId: ProjectId; path: string }) => unwrapResult(commands.treeToggle(input.projectId, input.path))

export const revealTreeNode = (input: { projectId: ProjectId; path: string }) => unwrapResult(commands.treeReveal(input.projectId, input.path))

export const refreshTreeDir = (input: { projectId: ProjectId; dir: string }) => unwrapResult(commands.treeRefresh(input.projectId, input.dir))
