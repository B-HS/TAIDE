import { useEffect, useLayoutEffect, useRef, type FC, type PropsWithChildren } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { currentThemeQueryOptions, useThemePreviewValue } from '@entities/theme/theme.query'
import { currentLocaleQueryOptions } from '@entities/locale/locale.query'
import { assemblePluginGrammarRegistrations } from '@entities/plugin/plugin-grammar'
import { readPluginGrammar } from '@entities/plugin/plugin.ipc'
import { pluginListQueryOptions } from '@entities/plugin/plugin.query'
import { applyThemeVariables } from '@shared/lib/theme-variables'
import { applyWindowAppearance } from '@shared/lib/window-appearance'
import { registerPluginLanguages } from '@shared/lib/monaco/register-plugin-languages'
import { applyShikiTheme, initShiki } from '@shared/lib/shiki/shiki-monaco'
import { isWindowReadyToReveal, useRevealWindow } from '@shared/hooks/use-reveal-window'
import { subscribeSystemTheme } from '@shared/lib/system-appearance'
import { QUERY_KEY } from '@shared/constants/query-key'
import { STATUS_ERROR_BANNER_HEIGHT_PX, StatusErrorBanner } from '@shared/ui/status-error-banner'

export const ThemeProvider: FC<PropsWithChildren> = ({ children }) => {
    const hasBootstrappedPluginGrammarsRef = useRef(false)

    const { data: resolvedTheme, isFetched, isError, refetch } = useQuery(currentThemeQueryOptions())
    const { isFetched: isLocaleFetched, isError: isLocaleError } = useQuery(currentLocaleQueryOptions())
    const { data: plugins } = useQuery(pluginListQueryOptions())
    const previewTheme = useThemePreviewValue()
    const queryClient = useQueryClient()
    const { t } = useTranslation()

    const theme = previewTheme ?? resolvedTheme

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

    /**
     * `pluginListQueryOptions` sources the list through the shared entities query cache instead of
     * a raw `listPlugins()` fetch (contract F1#2) — installs/uninstalls elsewhere in the app
     * already keep that cache current via `plugin.query.ts`'s mutations. The bootstrap-once guard
     * mirrors this effect's previous `[]`-deps behavior: shiki only needs the grammar set assembled
     * once at boot, not on every later plugin-list change (those already trigger their own
     * `reinitShiki` through the mutations' `onSuccess`).
     */
    useEffect(() => {
        if (!plugins || hasBootstrappedPluginGrammarsRef.current) return
        hasBootstrappedPluginGrammarsRef.current = true
        registerPluginLanguages(plugins)
        void assemblePluginGrammarRegistrations(plugins, readPluginGrammar)
            .then(initShiki)
            .catch((error: unknown) => console.error('[shiki] failed to initialize highlighter', error))
    }, [plugins])

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
