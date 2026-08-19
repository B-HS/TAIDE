import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getSettings, setThemeId, updateSettings } from '@entities/settings/settings.ipc'
import {
    LOCALE_AFFECTING_SETTINGS_FIELDS,
    REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS,
    THEME_AFFECTING_SETTINGS_FIELDS,
    settingsPatchTouchesFields,
} from '@entities/settings/settings.constant'

export const settingsQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.SETTINGS.CURRENT, queryFn: getSettings, staleTime: Infinity })

export const useUpdateSettings = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: updateSettings,
        onSuccess: (settings, patch) => {
            queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)
            if (settingsPatchTouchesFields(patch, THEME_AFFECTING_SETTINGS_FIELDS))
                void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
            if (settingsPatchTouchesFields(patch, LOCALE_AFFECTING_SETTINGS_FIELDS))
                void queryClient.invalidateQueries({ queryKey: QUERY_KEY.LOCALE.ALL })
            if (settingsPatchTouchesFields(patch, REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS))
                void queryClient.invalidateQueries({ queryKey: QUERY_KEY.REMOTE.STATUS })
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
