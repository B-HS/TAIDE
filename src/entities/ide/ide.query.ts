import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { getIdeStatus, resolveIdeDiff } from '@entities/ide/ide.ipc'

export const ideStatusQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.IDE.STATUS, queryFn: getIdeStatus })

export const useIdeStatusSync = () => {
    const queryClient = useQueryClient()
    useTauriEvent(events.ideStatusChanged, ({ payload }) => {
        queryClient.setQueryData(QUERY_KEY.IDE.STATUS, payload.status)
    })
}

export const useResolveIdeDiff = () => useMutation({ mutationFn: resolveIdeDiff })
