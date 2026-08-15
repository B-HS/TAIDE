type Listener = () => void

const openListeners = new Set<Listener>()

export const requestOpenTaskRunner = () => {
    for (const listener of openListeners) listener()
}

export const subscribeOpenTaskRunner = (listener: Listener) => {
    openListeners.add(listener)
    return () => {
        openListeners.delete(listener)
    }
}
