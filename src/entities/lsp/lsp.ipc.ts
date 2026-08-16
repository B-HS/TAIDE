import { Channel } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { commands } from '@shared/api/bindings'
import type { LspServerId, ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

/**
 * Spawns (or, for a `sharesSessions` server, joins) a language-server session. `owner` — this
 * window's OS label — scopes session reuse to the calling window: two *different* windows editing
 * the same project must never share one JSON-RPC connection, since each window's LSP client is an
 * independent JS realm that unconditionally performs its own `initialize` handshake and mints its
 * own request ids on acquiring a session. See the `owner` field doc on Rust's
 * `lsp::commands::SessionEntry`.
 */
export const spawnLspSession = (input: { projectId: ProjectId; serverId: LspServerId; root: string; onMessage: (message: string) => void }) => {
    const channel = new Channel<string>()
    channel.onmessage = (message) => input.onMessage(message)
    const request = { projectId: input.projectId, serverId: input.serverId, root: input.root, owner: getCurrentWindow().label }
    return unwrapResult(commands.lspSpawn(request, channel))
}

export const sendLspMessage = (input: { sessionId: string; message: string }) => unwrapResult(commands.lspSend(input.sessionId, input.message))

export const stopLspSession = (sessionId: string, root?: string) => unwrapResult(commands.lspStop(sessionId, root ?? null, getCurrentWindow().label))

export const restartLspSession = (sessionId: string) => unwrapResult(commands.lspRestart(sessionId))

export const listLspSessions = (projectId: ProjectId) => unwrapResult(commands.lspSessions(projectId))

export const detectLspServers = () => unwrapResult(commands.lspDetectServers())

export const resolveLspRoot = (input: { serverId: LspServerId; filePath: string }) =>
    unwrapResult(commands.lspResolveRoot(input.serverId, input.filePath))

export const installLspServer = (serverId: LspServerId) => unwrapResult(commands.lspInstall(serverId))

export const cancelLspInstall = (serverId: LspServerId) => unwrapResult(commands.lspInstallCancel(serverId))
