import { useEffect, useLayoutEffect, type FC, type PropsWithChildren } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { currentThemeQueryOptions } from '@entities/theme/theme.query'
import { currentLocaleQueryOptions } from '@entities/locale/locale.query'
import { assemblePluginGrammarRegistrations } from '@entities/plugin/plugin-grammar'
import { listPlugins, readPluginGrammar } from '@entities/plugin/plugin.ipc'
import { applyThemeVariables } from '@shared/lib/theme-variables'
import { applyWindowAppearance } from '@shared/lib/window-appearance'
import { registerPluginLanguages } from '@shared/lib/monaco/register-plugin-languages'
import { applyShikiTheme, initShiki } from '@shared/lib/shiki/shiki-monaco'
import { isWindowReadyToReveal, useRevealWindow } from '@shared/hooks/use-reveal-window'
import { subscribeSystemTheme } from '@shared/lib/system-appearance'
import { QUERY_KEY } from '@shared/constants/query-key'
import { STATUS_ERROR_BANNER_HEIGHT_PX, StatusErrorBanner } from '@shared/ui/status-error-banner'

export const ThemeProvider: FC<PropsWithChildren> = ({ children }) => {
    const { data: theme, isFetched, isError, refetch } = useQuery(currentThemeQueryOptions())
    const { isFetched: isLocaleFetched, isError: isLocaleError } = useQuery(currentLocaleQueryOptions())
    const queryClient = useQueryClient()
    const { t } = useTranslation()

    useLayoutEffect(() => {
        if (!theme) return
        applyThemeVariables(theme.colors, document.documentElement)
        document.documentElement.dataset.themeType = theme.type
        document.documentElement.dataset.appearance = theme.type
        void applyShikiTheme(theme).catch((error: unknown) => console.error('[shiki] failed to apply theme', error))
        applyWindowAppearance(theme.type)
    }, [theme])

    useLayoutEffect(() => {
        if (!isFetched) return
        document.documentElement.dataset.themeReady = ''
    }, [isFetched])

    useEffect(() => subscribeSystemTheme(() => void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })), [queryClient])

    useEffect(() => {
        void listPlugins()
            .then((plugins) => {
                registerPluginLanguages(plugins)
                return assemblePluginGrammarRegistrations(plugins, readPluginGrammar)
            })
            .then(initShiki)
            .catch((error: unknown) => console.error('[shiki] failed to initialize highlighter', error))
    }, [])

    useRevealWindow(isWindowReadyToReveal(isFetched, isLocaleFetched))

    return (
        <>
            {isError && (
                <StatusErrorBanner
                    message={t('theme.loadFailed', { defaultValue: 'Failed to load the theme' })}
                    retryLabel={t('common.retry', { defaultValue: 'Retry' })}
                    onRetry={() => void refetch()}
                    stackOffsetPx={isLocaleError ? STATUS_ERROR_BANNER_HEIGHT_PX : 0}
                />
            )}
            {children}
        </>
    )
}
