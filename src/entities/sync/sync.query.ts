import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { connectSync, disconnectSync, downloadSync, getSyncStatus, uploadSync } from '@entities/sync/sync.ipc'

export const syncStatusQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.SYNC.STATUS, queryFn: getSyncStatus })

export const useConnectSync = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: connectSync,
        onSuccess: (status) => queryClient.setQueryData(QUERY_KEY.SYNC.STATUS, status),
    })
}

export const useDisconnectSync = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: disconnectSync,
        onSuccess: (status) => queryClient.setQueryData(QUERY_KEY.SYNC.STATUS, status),
    })
}

export const useUploadSync = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: uploadSync,
        onSuccess: (status) => queryClient.setQueryData(QUERY_KEY.SYNC.STATUS, status),
    })
}

export const useDownloadSync = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (force: boolean) => downloadSync(force),
        onSuccess: (result) => {
            if (result.kind === 'applied') queryClient.setQueryData(QUERY_KEY.SYNC.STATUS, result.status)
        },
    })
}
