import { useLayoutEffect, type FC, type PropsWithChildren } from 'react'
import { useQuery } from '@tanstack/react-query'
import { currentThemeQueryOptions } from '@entities/theme/theme.query'
import { applyThemeVariables } from '@shared/lib/theme-variables'
import { applyWindowAppearance } from '@shared/lib/window-appearance'
import { applyMonacoTheme } from '@shared/lib/monaco/theme'
import { monaco } from '@shared/lib/monaco/setup'
import { useRevealWindow } from '@shared/hooks/use-reveal-window'

export const ThemeProvider: FC<PropsWithChildren> = ({ children }) => {
    const { data: theme, isFetched } = useQuery(currentThemeQueryOptions())

    useLayoutEffect(() => {
        if (!theme) return
        applyThemeVariables(theme.colors, document.documentElement)
        document.documentElement.dataset.themeType = theme.type
        applyMonacoTheme(theme, monaco.editor)
        applyWindowAppearance(theme.type)
    }, [theme])

    useRevealWindow(isFetched)

    return children
}
