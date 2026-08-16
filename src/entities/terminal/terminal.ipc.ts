import { Channel, invoke } from '@tauri-apps/api/core'
import { commands } from '@shared/api/bindings'
import type { ProjectId, PtySpawnOptions } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

const SPAWN_COMMAND = 'pty_spawn'
const ATTACH_COMMAND = 'pty_attach'

const createByteChannel = (onData: (bytes: Uint8Array) => void) => {
    const channel = new Channel<ArrayBuffer>()
    channel.onmessage = (message) => onData(new Uint8Array(message))
    return channel
}

export const spawnPty = (options: PtySpawnOptions, onData: (bytes: Uint8Array) => void) =>
    invoke<string>(SPAWN_COMMAND, { opts: options, onData: createByteChannel(onData) })

export const attachPty = (sessionId: string, onData: (bytes: Uint8Array) => void) =>
    invoke<number>(ATTACH_COMMAND, { sessionId, onData: createByteChannel(onData) })

export const detachPty = (sessionId: string, subscriptionId: number) => unwrapResult(commands.ptyDetach(sessionId, subscriptionId))

export const writePty = (input: { sessionId: string; data: string }) => unwrapResult(commands.ptyWrite(input.sessionId, input.data))

export const resizePty = (input: { sessionId: string; cols: number; rows: number }) =>
    unwrapResult(commands.ptyResize(input.sessionId, input.cols, input.rows))

export const killPty = (sessionId: string) => unwrapResult(commands.ptyKill(sessionId))

export const setPtyPaused = (input: { sessionId: string; paused: boolean }) => unwrapResult(commands.ptySetPaused(input.sessionId, input.paused))

export const listTerminalSessions = (projectId: ProjectId) => unwrapResult(commands.terminalSessions(projectId))

export const listShellProfiles = () => unwrapResult(commands.shellProfiles())

export const resolveTerminalPath = (input: { path: string; cwd: string }) => unwrapResult(commands.resolveTerminalPath(input.path, input.cwd))
