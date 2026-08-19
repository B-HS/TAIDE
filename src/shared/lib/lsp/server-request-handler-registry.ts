export type ServerRequestHandler = (params: unknown) => Promise<unknown>

type ConfigurationItem = { scopeUri?: string; section?: string }
type ConfigurationParams = { items: ConfigurationItem[] }

const isConfigurationParams = (params: unknown): params is ConfigurationParams =>
    typeof params === 'object' && params !== null && Array.isArray((params as ConfigurationParams).items)

const handlers = new Map<string, ServerRequestHandler>()

/**
 * The returned dispose function only removes `handler` itself, not "whatever is currently
 * registered under `method`" — mirroring `client.ts`'s instance-scoped `registerRequestHandler`
 * (F7#12). Without the identity check, a stale dispose closure from an *earlier* registration
 * could race a *later* one for the same method (e.g. a reinitializing session re-registering a
 * default handler while the previous session's own disposal is still in flight) and delete the
 * new, still-wanted handler instead of a no-op.
 */
export const registerServerRequestHandler = (method: string, handler: ServerRequestHandler) => {
    handlers.set(method, handler)
    return () => {
        if (handlers.get(method) === handler) unregisterServerRequestHandler(method)
    }
}

export const unregisterServerRequestHandler = (method: string) => {
    handlers.delete(method)
}

export const getServerRequestHandler = (method: string) => handlers.get(method)

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
