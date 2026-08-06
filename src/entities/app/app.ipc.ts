import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getAppInfo = () => unwrapResult(commands.appGetInfo())
