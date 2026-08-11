import { queryOptions, useMutation } from '@tanstack/react-query'
import { getRemoteStatus, issueRemoteLink, revokeRemoteSessions } from '@entities/remote/remote.ipc'
import { QUERY_KEY } from '@shared/constants/query-key'

const REMOTE_STATUS_REFETCH_MS = 5_000

export const remoteStatusQueryOptions = () =>
    queryOptions({ queryKey: QUERY_KEY.REMOTE.STATUS, queryFn: getRemoteStatus, refetchInterval: REMOTE_STATUS_REFETCH_MS })

export const useIssueRemoteLink = () => useMutation({ mutationFn: issueRemoteLink })

export const useRevokeRemoteSessions = () => useMutation({ mutationFn: revokeRemoteSessions })
