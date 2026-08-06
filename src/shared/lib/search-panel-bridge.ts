type OpenSearchPanelListener = () => void

const listeners = new Set<OpenSearchPanelListener>()

export const requestOpenSearchPanel = () => {
    for (const listener of listeners) listener()
}

export const subscribeOpenSearchPanel = (listener: OpenSearchPanelListener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
