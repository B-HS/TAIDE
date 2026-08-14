export type ServerRequestHandler = (params: unknown) => Promise<unknown>

type ConfigurationItem = { scopeUri?: string; section?: string }
type ConfigurationParams = { items: ConfigurationItem[] }

const isConfigurationParams = (params: unknown): params is ConfigurationParams =>
    typeof params === 'object' && params !== null && Array.isArray((params as ConfigurationParams).items)

const handlers = new Map<string, ServerRequestHandler>()

export const registerServerRequestHandler = (method: string, handler: ServerRequestHandler) => {
    handlers.set(method, handler)
    return () => unregisterServerRequestHandler(method)
}

export const unregisterServerRequestHandler = (method: string) => {
    handlers.delete(method)
}

export const getServerRequestHandler = (method: string) => handlers.get(method)

type RefreshListener = () => void

const codeLensRefreshListeners = new Set<RefreshListener>()

export const subscribeCodeLensRefresh = (listener: RefreshListener) => {
    codeLensRefreshListeners.add(listener)
    return () => codeLensRefreshListeners.delete(listener)
}

registerServerRequestHandler('workspace/configuration', async (params) => {
    const items = isConfigurationParams(params) ? params.items : []
    return items.map(() => null)
})

registerServerRequestHandler('client/registerCapability', async () => null)

/**
 * Paired with `client/registerCapability` above — a server that dynamically registers a
 * capability (e.g. gopls re-registering `didChangeWatchedFiles` after a config change) will
 * eventually ask to unregister it too. Without a handler, `client.ts` answers with `-32601
 * MethodNotFound`, which some servers log as an error even though nothing is actually broken.
 */
registerServerRequestHandler('client/unregisterCapability', async () => null)

registerServerRequestHandler('window/workDoneProgress/create', async () => null)

registerServerRequestHandler('workspace/codeLens/refresh', async () => {
    codeLensRefreshListeners.forEach((listener) => listener())
    return null
})
