import { commands } from '@shared/api/bindings'
import type { AppDataPathKind } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getSystemUsage = () => unwrapResult(commands.systemUsageGet())

export const systemOpenPath = (path: string) => unwrapResult(commands.systemOpenPath(path))

export const systemRevealPath = (path: string) => unwrapResult(commands.systemRevealPath(path))

export const systemOpenInBrowser = (path: string) => unwrapResult(commands.systemOpenInBrowser(path))

export const systemOpenAppDataPath = (kind: AppDataPathKind) => unwrapResult(commands.systemOpenAppDataPath(kind))
