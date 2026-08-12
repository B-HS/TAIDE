import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ResolvedTheme } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { deleteTheme, getCurrentTheme, getTheme, listThemes, saveTheme } from '@entities/theme/theme.ipc'
import { readSystemTheme } from '@shared/lib/system-appearance'
import { diffThemeValues } from '@shared/lib/theme-draft'
import type { ThemeDraft } from '@shared/lib/theme-draft'

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

export const useThemePreview = () => {
    const queryClient = useQueryClient()
    return {
        setPreview: (draft: ThemeDraft) => {
            const baseTheme = queryClient.getQueryData<ResolvedTheme>(QUERY_KEY.THEME.DETAIL(draft.extendsId))
            queryClient.setQueryData(QUERY_KEY.THEME.CURRENT, toResolvedThemeFromDraft(draft, baseTheme?.tokenColors))
        },
        clearPreview: () => void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.CURRENT }),
    }
}
