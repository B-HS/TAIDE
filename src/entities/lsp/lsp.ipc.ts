import { Channel } from '@tauri-apps/api/core'
import { commands } from '@shared/api/bindings'
import type { LspServerId, ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const spawnLspSession = (input: { projectId: ProjectId; serverId: LspServerId; root: string; onMessage: (message: string) => void }) => {
    const channel = new Channel<string>()
    channel.onmessage = (message) => input.onMessage(message)
    return unwrapResult(commands.lspSpawn(input.projectId, input.serverId, input.root, channel))
}

export const sendLspMessage = (input: { sessionId: string; message: string }) => unwrapResult(commands.lspSend(input.sessionId, input.message))

export const stopLspSession = (sessionId: string, root?: string) => unwrapResult(commands.lspStop(sessionId, root ?? null))

export const restartLspSession = (sessionId: string) => unwrapResult(commands.lspRestart(sessionId))

export const listLspSessions = (projectId: ProjectId) => unwrapResult(commands.lspSessions(projectId))

export const detectLspServers = () => unwrapResult(commands.lspDetectServers())

export const resolveLspRoot = (input: { serverId: LspServerId; filePath: string }) =>
    unwrapResult(commands.lspResolveRoot(input.serverId, input.filePath))

export const installLspServer = (serverId: LspServerId) => unwrapResult(commands.lspInstall(serverId))

export const cancelLspInstall = (serverId: LspServerId) => unwrapResult(commands.lspInstallCancel(serverId))
