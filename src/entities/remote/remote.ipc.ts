import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getRemoteStatus = () => unwrapResult(commands.remoteStatus())

export const issueRemoteLink = () => unwrapResult(commands.remoteIssueLink())

export const revokeRemoteSessions = () => unwrapResult(commands.remoteRevokeSessions())

export const setRemotePassword = (password: string) => unwrapResult(commands.remoteSetPassword(password))

export const clearRemotePassword = () => unwrapResult(commands.remoteClearPassword())
