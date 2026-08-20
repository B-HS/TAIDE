import { commands } from '@shared/api/bindings'
import type { ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listProjects = () => unwrapResult(commands.projectList())

export const listRecentProjects = () => unwrapResult(commands.projectListRecent())

export const getProject = (projectId: ProjectId) => unwrapResult(commands.projectGet(projectId))

export const getActiveProjectId = () => unwrapResult(commands.projectGetActive())

export const openProject = (path: string) => unwrapResult(commands.projectOpen(path))

export const closeProject = (projectId: ProjectId) => unwrapResult(commands.projectClose(projectId))

export const activateProject = (projectId: ProjectId) => unwrapResult(commands.projectActivate(projectId))

export const reorderProjects = (ids: ProjectId[]) => unwrapResult(commands.projectReorder(ids))
