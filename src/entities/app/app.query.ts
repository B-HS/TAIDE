import { queryOptions } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getAppInfo } from '@entities/app/app.ipc'

export const appInfoQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.APP.INFO, queryFn: getAppInfo, staleTime: Infinity })
