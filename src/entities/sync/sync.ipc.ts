import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getSyncStatus = () => unwrapResult(commands.syncStatus())

export const connectSync = (pat: string) => unwrapResult(commands.syncConnect(pat))

export const disconnectSync = () => unwrapResult(commands.syncDisconnect())

export const uploadSync = () => unwrapResult(commands.syncUpload())

export const downloadSync = (force: boolean) => unwrapResult(commands.syncDownload(force))
