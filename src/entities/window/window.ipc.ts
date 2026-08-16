import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const setWindowFullscreen = (fullscreen: boolean) => unwrapResult(commands.windowSetFullscreen(fullscreen))
