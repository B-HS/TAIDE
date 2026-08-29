import { Channel, invoke } from '@tauri-apps/api/core'
import { commands } from '@shared/api/bindings'
import type { ProjectId, PtySpawnOptions } from '@shared/api/bindings'
import { IpcError, isAppError, unwrapResult } from '@shared/api/unwrap-result'
import { enqueueSessionWrite } from '@entities/terminal/session-write-order'

const SPAWN_COMMAND = 'pty_spawn'
const ATTACH_COMMAND = 'pty_attach'

const createByteChannel = (onData: (bytes: Uint8Array) => void) => {
    const channel = new Channel<ArrayBuffer>()
    channel.onmessage = (message) => onData(new Uint8Array(message))
    return channel
}

/**
 * `spawnPty`/`attachPty` carry a `Channel` argument for the byte stream, so they call raw `invoke`
 * instead of the generated `commands.*` (which `unwrapResult` expects an `IpcResult` envelope
 * from). A rejection here is still the backend's bare `AppError`, though — normalize it into an
 * `IpcError` the same way `unwrapResult` does, so `describeIpcError`/`useIpcErrorMessage` resolve
 * it through the locale catalog instead of falling through to `String(error)`.
 */
const invokeRaw = async <T>(command: string, args: Record<string, unknown>) => {
    try {
        return await invoke<T>(command, args)
    } catch (error) {
        if (isAppError(error)) throw new IpcError(error)
        throw error instanceof Error ? error : new Error(String(error))
    }
}

export const spawnPty = (options: PtySpawnOptions, onData: (bytes: Uint8Array) => void) =>
    invokeRaw<string>(SPAWN_COMMAND, { opts: options, onData: createByteChannel(onData) })

export const attachPty = (sessionId: string, onData: (bytes: Uint8Array) => void) =>
    invokeRaw<number>(ATTACH_COMMAND, { sessionId, onData: createByteChannel(onData) })

export const detachPty = (sessionId: string, subscriptionId: number) => unwrapResult(commands.ptyDetach(sessionId, subscriptionId))

/** Serialized per session ({@link enqueueSessionWrite}) — the backend no longer preserves call order across concurrent `pty_write` invocations. */
export const writePty = (input: { sessionId: string; data: string }) =>
    enqueueSessionWrite(input.sessionId, () => unwrapResult(commands.ptyWrite(input.sessionId, input.data)))

export const resizePty = (input: { sessionId: string; cols: number; rows: number }) =>
    unwrapResult(commands.ptyResize(input.sessionId, input.cols, input.rows))

export const killPty = (sessionId: string) => unwrapResult(commands.ptyKill(sessionId))

export const setPtyPaused = (input: { sessionId: string; paused: boolean }) => unwrapResult(commands.ptySetPaused(input.sessionId, input.paused))

export const listTerminalSessions = (projectId: ProjectId) => unwrapResult(commands.terminalSessions(projectId))

export const listShellProfiles = () => unwrapResult(commands.shellProfiles())

export const resolveTerminalPath = (input: { path: string; cwd: string }) => unwrapResult(commands.resolveTerminalPath(input.path, input.cwd))
