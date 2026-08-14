import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { i18next } from '@shared/i18n/i18n'
import { clearRemotePassword, getRemoteStatus, issueRemoteLink, revokeRemoteSessions, setRemotePassword } from '@entities/remote/remote.ipc'
import { QUERY_KEY } from '@shared/constants/query-key'

const REMOTE_STATUS_REFETCH_MS = 5_000

export const remoteStatusQueryOptions = () =>
    queryOptions({ queryKey: QUERY_KEY.REMOTE.STATUS, queryFn: getRemoteStatus, refetchInterval: REMOTE_STATUS_REFETCH_MS })

export const useIssueRemoteLink = () => useMutation({ mutationFn: issueRemoteLink })

export const useRevokeRemoteSessions = () => useMutation({ mutationFn: revokeRemoteSessions })

export const useSetRemotePassword = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: setRemotePassword,
        onSuccess: () => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.REMOTE.STATUS })
            toast.success(i18next.t('remote.passwordSessionsRevoked'))
        },
    })
}

export const useClearRemotePassword = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: clearRemotePassword,
        onSuccess: () => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.REMOTE.STATUS })
            toast.success(i18next.t('remote.passwordSessionsRevoked'))
        },
    })
}
