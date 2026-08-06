import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listThemes = () => unwrapResult(commands.themeList())

export const getTheme = (themeId: string) => unwrapResult(commands.themeGet(themeId))

export const getCurrentTheme = () => unwrapResult(commands.themeGetCurrent())
