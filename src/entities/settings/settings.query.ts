import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { i18next } from '@shared/i18n/i18n'
import { QUERY_KEY } from '@shared/constants/query-key'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { getSettings, setThemeId, updateSettings } from '@entities/settings/settings.ipc'
import {
    LOCALE_AFFECTING_SETTINGS_FIELDS,
    REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS,
    THEME_AFFECTING_SETTINGS_FIELDS,
    settingsPatchTouchesFields,
} from '@entities/settings/settings.constant'

export const settingsQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.SETTINGS.CURRENT, queryFn: getSettings, staleTime: Infinity })

/**
 * Every settings write in the app goes through the two mutations below, and neither reported a
 * failure: `settings_update`/`settings_set_theme` can fail for reasons the user cannot see (the
 * `settings.json` write bounced, a schema-invalid `settings.json` on disk, a remote session denied
 * the command), and each toggle/field is rendered straight from the cached `Settings`, so a failed
 * write simply left the control snapped back to its old value with no explanation (audit §4-B B15).
 * Reported here rather than at the ~20 call sites so a new settings control cannot forget it;
 * `describeIpcError` surfaces the backend's own localized reason when it has one, with the generic
 * title as the fallback description.
 */
const toastSettingsSaveFailure = (error: unknown) => toast.error(i18next.t('settings.saveFailed'), { description: describeIpcError(error) })

export const useUpdateSettings = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: updateSettings,
        onError: toastSettingsSaveFailure,
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
        onError: toastSettingsSaveFailure,
        onSuccess: (settings) => {
            queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)
            queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
        },
    })
}
