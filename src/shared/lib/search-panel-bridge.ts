export type SearchPanelScope = { includeGlob: string }

type OpenSearchPanelListener = (scope: SearchPanelScope | null) => void

const listeners = new Set<OpenSearchPanelListener>()

export const requestOpenSearchPanel = (scope: SearchPanelScope | null = null) => {
    for (const listener of listeners) listener(scope)
}

export const subscribeOpenSearchPanel = (listener: OpenSearchPanelListener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
