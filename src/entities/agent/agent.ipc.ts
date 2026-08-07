import { commands } from '@shared/api/bindings'
import type { ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listProjectAgents = (projectId: ProjectId) => unwrapResult(commands.agentList(projectId))

export const releaseWaitMarker = (marker: string) => unwrapResult(commands.agentReleaseMarker(marker))

export const getCliInstallStatus = () => unwrapResult(commands.agentCliStatus())

export const getAgentHooksStatus = (projectId: ProjectId) => unwrapResult(commands.agentHooksStatus(projectId))

export const installAgentHooks = (projectId: ProjectId) => unwrapResult(commands.agentHooksInstall(projectId))

export const uninstallAgentHooks = (projectId: ProjectId) => unwrapResult(commands.agentHooksUninstall(projectId))
