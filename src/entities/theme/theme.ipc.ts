import type { Theme } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listThemes = () => unwrapResult(commands.themeList())

export const getTheme = (themeId: string) => unwrapResult(commands.themeGet(themeId))

export const getCurrentTheme = (systemTheme: string) => unwrapResult(commands.themeGetCurrent(systemTheme))

export const saveTheme = (theme: Theme) => unwrapResult(commands.themeSave(theme))

export const deleteTheme = (themeId: string) => unwrapResult(commands.themeDelete(themeId))
