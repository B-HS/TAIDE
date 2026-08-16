import type { QueryClient } from '@tanstack/react-query'
import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { LoadedPlugin } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { registerPluginLanguages } from '@shared/lib/monaco/register-plugin-languages'
import { reinitShiki } from '@shared/lib/shiki/shiki-monaco'
import { installPlugin, listPlugins, readPluginGrammar, reloadPlugins, uninstallPlugin } from '@entities/plugin/plugin.ipc'
import { assemblePluginGrammarRegistrations } from '@entities/plugin/plugin-grammar'

export const pluginListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.PLUGIN.LIST, queryFn: listPlugins })

/**
 * Caches `plugins` as the current `PLUGIN.LIST`, registers any of their language ids monaco hasn't
 * seen yet (`registerPluginLanguages`), and rebuilds the shared shiki highlighter against the
 * refreshed grammar set — the three steps every plugin-list-changing mutation below needs, in the
 * order that keeps the editor's language support (monaco language id → shiki grammar) consistent
 * with whatever is actually enabled (contract §3.4/C1).
 */
const applyPluginList = async (queryClient: QueryClient, plugins: LoadedPlugin[]) => {
    queryClient.setQueryData(QUERY_KEY.PLUGIN.LIST, plugins)
    registerPluginLanguages(plugins)
    await reinitShiki(await assemblePluginGrammarRegistrations(plugins, readPluginGrammar))
}

/**
 * Re-fetches the full plugin list and applies it — used by mutations (`plugin_install`,
 * `vsix_import_plugin`) whose Rust command only returns the single newly-installed plugin, not the
 * refreshed list `plugin_reload`/`plugin_uninstall` already hand back directly.
 */
export const refetchAndApplyPluginList = async (queryClient: QueryClient) => applyPluginList(queryClient, await listPlugins())

export const useReloadPlugins = () => {
    const queryClient = useQueryClient()
    return useMutation({ mutationFn: reloadPlugins, onSuccess: (plugins) => applyPluginList(queryClient, plugins) })
}

export const useInstallPlugin = () => {
    const queryClient = useQueryClient()
    return useMutation({ mutationFn: installPlugin, onSuccess: () => refetchAndApplyPluginList(queryClient) })
}

export const useUninstallPlugin = () => {
    const queryClient = useQueryClient()
    return useMutation({ mutationFn: uninstallPlugin, onSuccess: (plugins) => applyPluginList(queryClient, plugins) })
}
