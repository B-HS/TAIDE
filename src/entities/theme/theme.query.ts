import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { deleteTheme, getCurrentTheme, getTheme, listThemes, saveTheme } from '@entities/theme/theme.ipc'
import { readSystemTheme } from '@shared/lib/system-appearance'

export const themeListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.THEME.LIST, queryFn: listThemes })

export const currentThemeQueryOptions = () =>
    queryOptions({ queryKey: QUERY_KEY.THEME.CURRENT, queryFn: () => getCurrentTheme(readSystemTheme()), staleTime: Infinity })

export const themeQueryOptions = (themeId: string) => queryOptions({ queryKey: QUERY_KEY.THEME.DETAIL(themeId), queryFn: () => getTheme(themeId) })

export const useSaveTheme = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: saveTheme,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL }),
    })
}

export const useDeleteTheme = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: deleteTheme,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL }),
    })
}
