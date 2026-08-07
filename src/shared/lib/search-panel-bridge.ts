export type SearchPanelRequest = {
    includeGlob: string | null
    seedText: string | null
    openReplace: boolean
}

type OpenSearchPanelListener = (request: SearchPanelRequest) => void

const listeners = new Set<OpenSearchPanelListener>()

export const requestOpenSearchPanel = (request: Partial<SearchPanelRequest> = {}) => {
    const payload: SearchPanelRequest = { includeGlob: null, seedText: null, openReplace: false, ...request }
    for (const listener of listeners) listener(payload)
}

export const subscribeOpenSearchPanel = (listener: OpenSearchPanelListener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
