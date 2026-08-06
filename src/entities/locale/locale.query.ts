import { queryOptions } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getCurrentLocale, listLocales } from '@entities/locale/locale.ipc'

const systemLanguage = () => (typeof navigator === 'undefined' ? '' : navigator.language)

export const localeListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.LOCALE.LIST, queryFn: listLocales, staleTime: Infinity })

export const currentLocaleQueryOptions = () =>
    queryOptions({ queryKey: QUERY_KEY.LOCALE.CURRENT, queryFn: () => getCurrentLocale(systemLanguage()), staleTime: Infinity })
