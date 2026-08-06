import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listLocales = () => unwrapResult(commands.localeList())

export const getLocale = (localeId: string) => unwrapResult(commands.localeGet(localeId))

export const getCurrentLocale = (systemLanguage: string) => unwrapResult(commands.localeGetCurrent(systemLanguage))
