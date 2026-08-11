import { useEffect, useLayoutEffect, type FC, type PropsWithChildren } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { currentThemeQueryOptions } from '@entities/theme/theme.query'
import { applyThemeVariables } from '@shared/lib/theme-variables'
import { applyWindowAppearance } from '@shared/lib/window-appearance'
import { applyMonacoTheme } from '@shared/lib/monaco/theme'
import { monaco } from '@shared/lib/monaco/setup'
import { useRevealWindow } from '@shared/hooks/use-reveal-window'
import { subscribeSystemTheme } from '@shared/lib/system-appearance'
import { QUERY_KEY } from '@shared/constants/query-key'

export const ThemeProvider: FC<PropsWithChildren> = ({ children }) => {
    const { data: theme, isFetched } = useQuery(currentThemeQueryOptions())
    const queryClient = useQueryClient()

    useLayoutEffect(() => {
        if (!theme) return
        applyThemeVariables(theme.colors, document.documentElement)
        document.documentElement.dataset.themeType = theme.type
        document.documentElement.dataset.appearance = theme.type
        applyMonacoTheme(theme, monaco.editor)
        applyWindowAppearance(theme.type)
    }, [theme])

    useLayoutEffect(() => {
        if (!isFetched) return
        document.documentElement.dataset.themeReady = ''
    }, [isFetched])

    useEffect(() => subscribeSystemTheme(() => void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })), [queryClient])

    useRevealWindow(isFetched)

    return children
}
