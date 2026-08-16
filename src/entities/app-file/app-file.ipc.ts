import { commands } from '@shared/api/bindings'
import type { AppFileTarget } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const readAppFile = (target: AppFileTarget) => unwrapResult(commands.appFileRead(target))

export const writeAppFile = (input: { target: AppFileTarget; content: string }) => unwrapResult(commands.appFileWrite(input.target, input.content))
