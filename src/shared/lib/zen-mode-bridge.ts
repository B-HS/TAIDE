type Listener = () => void

const toggleListeners = new Set<Listener>()

export const requestToggleZenMode = () => {
    for (const listener of toggleListeners) listener()
}

export const subscribeToggleZenMode = (listener: Listener) => {
    toggleListeners.add(listener)
    return () => {
        toggleListeners.delete(listener)
    }
}
