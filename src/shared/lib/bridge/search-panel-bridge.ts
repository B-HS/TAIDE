import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

export type SearchPanelRequest = {
    includeGlob: string | null
    /**
     * A project-relative directory the run is confined to — the Explorer's "폴더에서 찾기". Carried
     * separately from `includeGlob` because the backend treats the two differently: a glob only
     * filters what the walk already produced, and the walk prunes `IGNORED_DIR_NAMES` first, so
     * right-clicking `node_modules` and searching it reported a truthful-looking zero results
     * (audit wave 2 #23). A scope moves the walk root instead and suspends that pruning inside it.
     */
    scopeDir: string | null
    seedText: string | null
    openReplace: boolean
}

const openSearchPanelBridge = createFireAndForgetBridge<SearchPanelRequest>()

export const requestOpenSearchPanel = (request: Partial<SearchPanelRequest> = {}) => {
    const payload: SearchPanelRequest = { includeGlob: null, scopeDir: null, seedText: null, openReplace: false, ...request }
    openSearchPanelBridge.publish(payload)
}

export const subscribeOpenSearchPanel = openSearchPanelBridge.subscribe
