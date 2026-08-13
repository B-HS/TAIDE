import { queryOptions } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { SYSTEM_USAGE_POLL_INTERVAL_MS } from '@shared/constants/system-usage'
import { getSystemUsage, getSystemUsageBreakdown } from '@entities/system/system.ipc'

export const systemUsageQueryOptions = (enabled: boolean) =>
    queryOptions({
        queryKey: QUERY_KEY.SYSTEM.USAGE,
        queryFn: getSystemUsage,
        enabled,
        refetchInterval: SYSTEM_USAGE_POLL_INTERVAL_MS,
        refetchIntervalInBackground: false,
        staleTime: 0,
        gcTime: 0,
    })

export const systemUsageBreakdownQueryOptions = (enabled: boolean) =>
    queryOptions({
        queryKey: QUERY_KEY.SYSTEM.USAGE_BREAKDOWN,
        queryFn: getSystemUsageBreakdown,
        enabled,
        refetchInterval: SYSTEM_USAGE_POLL_INTERVAL_MS,
        refetchIntervalInBackground: false,
        staleTime: 0,
        gcTime: 0,
    })
