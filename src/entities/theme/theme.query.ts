import { useSyncExternalStore } from 'react'
import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ResolvedTheme } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { deleteTheme, getCurrentTheme, getTheme, listThemes, saveTheme } from '@entities/theme/theme.ipc'
import { readSystemTheme } from '@shared/lib/system-appearance'
import { diffThemeValues } from '@shared/lib/theme-draft'
import type { ThemeDraft } from '@shared/lib/theme-draft'
import { createExternalStoreBridge } from '@shared/lib/external-store-bridge'

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

const toResolvedThemeFromDraft = (draft: ThemeDraft, baseTokenColors: ResolvedTheme['tokenColors']): ResolvedTheme => ({
    id: draft.id,
    name: draft.name,
    type: draft.themeType,
    colors: draft.current.colors,
    syntax: draft.current.syntax,
    terminal: draft.current.terminal,
    tokenColors: baseTokenColors,
    syntaxOverrides: Object.keys(diffThemeValues(draft.base, draft.current).syntax),
    warnings: [],
})

/**
 * Holds the theme editor's live preview override outside the query cache (contract F1#11≡R5#10):
 * a draft-in-progress is client-only UI state, not a server fact, so it must never overwrite
 * `QUERY_KEY.THEME.CURRENT` — that key is reserved for what `theme_get_current` actually resolved.
 * `ThemeProvider` reads this alongside {@link currentThemeQueryOptions} and prefers it when set.
 */
const themePreviewStore = createExternalStoreBridge<ResolvedTheme | null>(null)

export const useThemePreviewValue = () => useSyncExternalStore(themePreviewStore.subscribe, themePreviewStore.getSnapshot)

export const useThemePreview = () => {
    const queryClient = useQueryClient()
    return {
        setPreview: (draft: ThemeDraft) => {
            const baseTheme = queryClient.getQueryData<ResolvedTheme>(QUERY_KEY.THEME.DETAIL(draft.extendsId))
            themePreviewStore.setValue(toResolvedThemeFromDraft(draft, baseTheme?.tokenColors))
        },
        clearPreview: () => themePreviewStore.setValue(null),
    }
}
