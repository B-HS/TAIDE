import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listFonts = () => unwrapResult(commands.fontList())
