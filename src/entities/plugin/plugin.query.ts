import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { reinitShiki } from '@shared/lib/shiki/shiki-monaco'
import { listPlugins, reloadPlugins } from '@entities/plugin/plugin.ipc'
import { loadPluginGrammarRegistrations } from '@entities/plugin/plugin-grammar'

export const pluginListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.PLUGIN.LIST, queryFn: listPlugins })

export const useReloadPlugins = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: reloadPlugins,
        onSuccess: async (plugins) => {
            queryClient.setQueryData(QUERY_KEY.PLUGIN.LIST, plugins)
            await reinitShiki(await loadPluginGrammarRegistrations())
        },
    })
}
