import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listPlugins = () => unwrapResult(commands.pluginList())

export const reloadPlugins = () => unwrapResult(commands.pluginReload())

export const installPlugin = (sourcePath: string) => unwrapResult(commands.pluginInstall(sourcePath))

export const uninstallPlugin = (pluginId: string) => unwrapResult(commands.pluginUninstall(pluginId))

export const readPluginGrammar = (pluginId: string, languageId: string) => unwrapResult(commands.pluginReadGrammar(pluginId, languageId))
