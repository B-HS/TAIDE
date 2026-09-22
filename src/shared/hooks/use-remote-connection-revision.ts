import { useSyncExternalStore } from 'react'
import { getRemoteConnectionRevision, subscribeRemoteConnection } from '@shared/lib/remote/connection-revision'

export const useRemoteConnectionRevision = () => useSyncExternalStore(subscribeRemoteConnection, getRemoteConnectionRevision)
