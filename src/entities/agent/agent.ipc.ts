import { commands } from '@shared/api/bindings'
import type { ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listProjectAgents = (projectId: ProjectId) => unwrapResult(commands.agentList(projectId))

export const releaseWaitMarker = (marker: string) => unwrapResult(commands.agentReleaseMarker(marker))

export const getCliInstallStatus = () => unwrapResult(commands.agentCliStatus())

export const installCliCommand = () => unwrapResult(commands.agentCliInstall())

export const uninstallCliCommand = () => unwrapResult(commands.agentCliUninstall())

export const pendingExternalOpens = () => unwrapResult(commands.agentPendingExternalOpens())

export const getAgentHooksStatus = (projectId: ProjectId, agentName: string) => unwrapResult(commands.agentHooksStatus(projectId, agentName))

export const installAgentHooks = (projectId: ProjectId, agentName: string) => unwrapResult(commands.agentHooksInstall(projectId, agentName))

export const uninstallAgentHooks = (projectId: ProjectId, agentName: string) => unwrapResult(commands.agentHooksUninstall(projectId, agentName))
