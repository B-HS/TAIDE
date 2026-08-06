import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { listPlugins, reloadPlugins } from '@entities/plugin/plugin.ipc'

export const pluginListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.PLUGIN.LIST, queryFn: listPlugins })

export const useReloadPlugins = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: reloadPlugins,
        onSuccess: (plugins) => queryClient.setQueryData(QUERY_KEY.PLUGIN.LIST, plugins),
    })
}
