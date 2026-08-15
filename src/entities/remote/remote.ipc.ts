import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

/** Mirrors `REMOTE_PASSWORD_MIN_LEN` (`src-tauri/src/domain/remote/types.rs`) — hand-kept in sync, since specta bindings only cover commands/types, not backend constants. */
export const REMOTE_PASSWORD_MIN_LEN = 8

export const getRemoteStatus = () => unwrapResult(commands.remoteStatus())

export const issueRemoteLink = () => unwrapResult(commands.remoteIssueLink())

export const revokeRemoteSessions = () => unwrapResult(commands.remoteRevokeSessions())

export const setRemotePassword = (password: string) => unwrapResult(commands.remoteSetPassword(password))

export const clearRemotePassword = () => unwrapResult(commands.remoteClearPassword())
