import { queryOptions } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getCurrentTheme, listThemes } from '@entities/theme/theme.ipc'

export const themeListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.THEME.LIST, queryFn: listThemes })

export const currentThemeQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.THEME.CURRENT, queryFn: getCurrentTheme, staleTime: Infinity })
