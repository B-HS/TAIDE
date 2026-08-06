import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getSettings, setThemeId, updateSettings } from '@entities/settings/settings.ipc'

export const settingsQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.SETTINGS.CURRENT, queryFn: getSettings, staleTime: Infinity })

export const useUpdateSettings = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: updateSettings,
        onSuccess: (settings) => {
            queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)
            queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
        },
    })
}

export const useSetThemeId = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: setThemeId,
        onSuccess: (settings) => {
            queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)
            queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
        },
    })
}
