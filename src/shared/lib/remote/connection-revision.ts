const listeners = new Set<() => void>()
let revision = 0

export const getRemoteConnectionRevision = () => revision

export const subscribeRemoteConnection = (listener: () => void) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}

export const publishRemoteReconnect = () => {
    revision += 1
    for (const listener of listeners) listener()
}
