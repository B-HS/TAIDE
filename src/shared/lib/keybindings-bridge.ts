type Listener = () => void

const openListeners = new Set<Listener>()

export const requestOpenKeybindingsEditor = () => {
    for (const listener of openListeners) listener()
}

export const subscribeOpenKeybindingsEditor = (listener: Listener) => {
    openListeners.add(listener)
    return () => {
        openListeners.delete(listener)
    }
}
