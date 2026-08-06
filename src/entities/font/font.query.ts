import { queryOptions } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { listFonts } from '@entities/font/font.ipc'

export const fontListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.FONT.LIST, queryFn: listFonts, staleTime: Infinity })
