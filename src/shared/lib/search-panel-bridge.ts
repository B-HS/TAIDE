import { createFireAndForgetBridge } from '@shared/lib/fire-and-forget-bridge'

export type SearchPanelRequest = {
    includeGlob: string | null
    seedText: string | null
    openReplace: boolean
}

const openSearchPanelBridge = createFireAndForgetBridge<SearchPanelRequest>()

export const requestOpenSearchPanel = (request: Partial<SearchPanelRequest> = {}) => {
    const payload: SearchPanelRequest = { includeGlob: null, seedText: null, openReplace: false, ...request }
    openSearchPanelBridge.publish(payload)
}

export const subscribeOpenSearchPanel = openSearchPanelBridge.subscribe
