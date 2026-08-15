type OpenFileHistoryListener = (path: string) => void

const listeners = new Set<OpenFileHistoryListener>()

export const requestOpenFileHistory = (path: string) => {
    for (const listener of listeners) listener(path)
}

export const subscribeOpenFileHistory = (listener: OpenFileHistoryListener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
