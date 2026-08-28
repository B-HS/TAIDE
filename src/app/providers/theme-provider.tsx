import { useEffect, useLayoutEffect, useRef, type FC, type PropsWithChildren } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { currentThemeQueryOptions, useThemePreviewValue } from '@entities/theme/theme.query'
import { currentLocaleQueryOptions } from '@entities/locale/locale.query'
import { assemblePluginGrammarRegistrations } from '@entities/plugin/plugin-grammar'
import { readPluginGrammar } from '@entities/plugin/plugin.ipc'
import { pluginListQueryOptions } from '@entities/plugin/plugin.query'
import type { ThemeType } from '@shared/api/bindings'
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
    const lastAppliedWindowAppearanceTypeRef = useRef<ThemeType | null>(null)

    const { data: resolvedTheme, isFetched, isError, refetch } = useQuery(currentThemeQueryOptions())
    const { isFetched: isLocaleFetched, isError: isLocaleError } = useQuery(currentLocaleQueryOptions())
    const { data: plugins } = useQuery(pluginListQueryOptions())
    const previewTheme = useThemePreviewValue()
    const queryClient = useQueryClient()
    const { t } = useTranslation()

    const theme = previewTheme ?? resolvedTheme

    /**
     * `applyWindowAppearance` is a native IPC round-trip that tao unconditionally re-applies
     * app-wide (`NSApplication.setAppearance:`) even when `theme.type` did not actually change —
     * it has no same-value short-circuit of its own (contract d-45 §0). A color-picker drag
     * re-runs this whole effect once per `pointermove` (via the rAF-coalesced preview) while
     * `theme.type` stays constant, so without this ref gate every one of those frames would still
     * fire a redundant native call and flood the main thread (contract d-45 §1#1).
     *
     * The ref is written before the call resolves, not after, so a same-`type` frame that lands
     * while the previous call is still in flight is also gated. If the call then rejects, the
     * `.catch` rolls the ref back to `null` (contract d-45 F-02) — otherwise a transient failure
     * would leave the guard believing `type` is already applied when the native appearance never
     * actually changed, permanently blocking every later retry of the same `type`.
     */
    useLayoutEffect(() => {
        if (!theme) return
        applyThemeVariables(theme.colors, document.documentElement)
        document.documentElement.dataset.themeType = theme.type
        document.documentElement.dataset.appearance = theme.type
        applyShikiTheme(theme)
        if (lastAppliedWindowAppearanceTypeRef.current !== theme.type) {
            lastAppliedWindowAppearanceTypeRef.current = theme.type
            applyWindowAppearance(theme.type).catch((error: unknown) => {
                lastAppliedWindowAppearanceTypeRef.current = null
                console.error('[window-appearance] failed to apply', error)
            })
        }
    }, [theme])

    useLayoutEffect(() => {
        if (!isFetched) return
        document.documentElement.dataset.themeReady = ''
    }, [isFetched])

    useEffect(() => subscribeSystemTheme(() => void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })), [queryClient])

    /**
     * Resets the window-appearance guard whenever this window regains focus. The native appearance
     * effect (`NSApplication.setAppearance:`) is app-wide, but `lastAppliedWindowAppearanceTypeRef`
     * is scoped to this webview realm (contract d-45 F-01): an auxiliary editor window mounts its
     * own `ThemeProvider` with its own ref starting at `null`, so it applies its own resolved theme's
     * appearance app-wide without this realm ever finding out — leaving this realm's ref pointing at
     * a `type` that is no longer what the native appearance actually is, and permanently blocking
     * this realm's guard from re-syncing until `theme.type` happens to change again. Clearing the ref
     * on focus — the moment this window's appearance is about to be looked at again — reopens that
     * re-sync path for the next time this effect runs.
     */
    useEffect(() => {
        const handleWindowFocus = () => {
            lastAppliedWindowAppearanceTypeRef.current = null
        }
        window.addEventListener('focus', handleWindowFocus)
        return () => window.removeEventListener('focus', handleWindowFocus)
    }, [])

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
