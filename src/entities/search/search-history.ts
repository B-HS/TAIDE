import { useQuery } from '@tanstack/react-query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'

export const SEARCH_HISTORY_LIMIT = 20

export const addRecentSearchTerm = (history: string[], term: string) => {
    const trimmed = term.trim()
    if (!trimmed) return history
    if (history[0] === trimmed) return history
    return [trimmed, ...history.filter((entry) => entry !== trimmed)].slice(0, SEARCH_HISTORY_LIMIT)
}

export const useRecentSearches = () => {
    const { data: settings } = useQuery(settingsQueryOptions())
    return settings?.recentSearches ?? []
}

export const useAddRecentSearch = () => {
    const recentSearches = useRecentSearches()
    const { mutate: updateSettings } = useUpdateSettings()

    return (term: string) => {
        const next = addRecentSearchTerm(recentSearches, term)
        if (next === recentSearches) return
        updateSettings({ ...emptySettingsPatch(), recentSearches: next })
    }
}
