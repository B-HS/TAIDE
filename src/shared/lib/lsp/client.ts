import type {
    IncrementalTextDocumentContentChangeEvent,
    InitializeResult,
    JsonRpcErrorResponse,
    JsonRpcId,
    JsonRpcNotification,
    JsonRpcRequest,
    JsonRpcResponse,
    JsonRpcResponseError,
    PublishDiagnosticsParams,
    ServerCapabilities,
    TextDocumentItem,
} from '@shared/lib/lsp/protocol'
import {
    JSON_RPC_ERROR_CODE,
    createRequestIdGenerator,
    isCapabilityEnabled,
    isJsonRpcErrorResponse,
    isJsonRpcNotification,
    isJsonRpcRequest,
    isJsonRpcResponse,
} from '@shared/lib/lsp/protocol'
import type { ServerRequestHandler } from '@shared/lib/lsp/server-request-handler-registry'
import { getServerRequestHandler } from '@shared/lib/lsp/server-request-handler-registry'

export class LspCapabilityNotSupportedError extends Error {
    constructor(method: string) {
        super(`server does not support method: ${method}`)
        this.name = 'LspCapabilityNotSupportedError'
    }
}

export class LspDocumentNotOpenError extends Error {
    constructor(uri: string) {
        super(`document not open: ${uri}`)
        this.name = 'LspDocumentNotOpenError'
    }
}

export type OutgoingMessage = JsonRpcRequest | JsonRpcNotification | JsonRpcResponse

export type LspClientDeps = {
    send: (message: OutgoingMessage) => void
    onNotification: (notification: JsonRpcNotification) => void
}

type PendingRequest = { resolve: (result: unknown) => void; reject: (error: JsonRpcResponseError | Error) => void }

const FEATURE_CAPABILITY_CHECKS: Record<string, (capabilities: ServerCapabilities) => boolean> = {
    'textDocument/completion': (capabilities) => capabilities.completionProvider !== undefined,
    'textDocument/hover': (capabilities) => capabilities.hoverProvider !== undefined && capabilities.hoverProvider !== false,
    'textDocument/definition': (capabilities) => capabilities.definitionProvider !== undefined && capabilities.definitionProvider !== false,
    'textDocument/references': (capabilities) => capabilities.referencesProvider !== undefined && capabilities.referencesProvider !== false,
    'textDocument/rename': (capabilities) => capabilities.renameProvider !== undefined && capabilities.renameProvider !== false,
    'textDocument/prepareRename': (capabilities) =>
        typeof capabilities.renameProvider === 'object' && capabilities.renameProvider.prepareProvider === true,
    'textDocument/formatting': (capabilities) =>
        capabilities.documentFormattingProvider !== undefined && capabilities.documentFormattingProvider !== false,
    'textDocument/rangeFormatting': (capabilities) =>
        capabilities.documentRangeFormattingProvider !== undefined && capabilities.documentRangeFormattingProvider !== false,
    'textDocument/signatureHelp': (capabilities) => capabilities.signatureHelpProvider !== undefined,
    'textDocument/inlayHint': (capabilities) => capabilities.inlayHintProvider !== undefined && capabilities.inlayHintProvider !== false,
    'textDocument/documentSymbol': (capabilities) =>
        capabilities.documentSymbolProvider !== undefined && capabilities.documentSymbolProvider !== false,
    'textDocument/documentHighlight': (capabilities) =>
        capabilities.documentHighlightProvider !== undefined && capabilities.documentHighlightProvider !== false,
    'textDocument/selectionRange': (capabilities) =>
        capabilities.selectionRangeProvider !== undefined && capabilities.selectionRangeProvider !== false,
    'textDocument/diagnostic': (capabilities) => capabilities.diagnosticProvider !== undefined,
    'textDocument/codeAction': (capabilities) => isCapabilityEnabled(capabilities.codeActionProvider),
    'codeAction/resolve': (capabilities) =>
        typeof capabilities.codeActionProvider === 'object' && capabilities.codeActionProvider.resolveProvider === true,
    'textDocument/codeLens': (capabilities) => capabilities.codeLensProvider !== undefined,
    'codeLens/resolve': (capabilities) => capabilities.codeLensProvider?.resolveProvider === true,
    'textDocument/foldingRange': (capabilities) => isCapabilityEnabled(capabilities.foldingRangeProvider),
    'textDocument/implementation': (capabilities) => isCapabilityEnabled(capabilities.implementationProvider),
    'textDocument/typeDefinition': (capabilities) => isCapabilityEnabled(capabilities.typeDefinitionProvider),
    'textDocument/declaration': (capabilities) => isCapabilityEnabled(capabilities.declarationProvider),
    'workspace/symbol': (capabilities) => isCapabilityEnabled(capabilities.workspaceSymbolProvider),
    'workspace/executeCommand': (capabilities) => capabilities.executeCommandProvider !== undefined,
}

export const createLspClient = (deps: LspClientDeps) => {
    const nextRequestId = createRequestIdGenerator()
    const pendingRequests = new Map<JsonRpcId, PendingRequest>()
    const diagnosticsListeners = new Set<(params: PublishDiagnosticsParams) => void>()
    const documentVersions = new Map<string, number>()
    const instanceRequestHandlers = new Map<string, ServerRequestHandler>()
    let serverCapabilities: ServerCapabilities | null = null

    const request = <TResult = unknown, TParams = unknown>(method: string, params?: TParams) =>
        new Promise<TResult>((resolve, reject) => {
            const capabilityCheck = FEATURE_CAPABILITY_CHECKS[method]
            if (capabilityCheck && (serverCapabilities === null || !capabilityCheck(serverCapabilities))) {
                reject(new LspCapabilityNotSupportedError(method))
                return
            }
            const id = nextRequestId()
            pendingRequests.set(id, { resolve: resolve as (result: unknown) => void, reject })
            deps.send({ jsonrpc: '2.0', id, method, params })
        })

    const notify = <TParams = unknown>(method: string, params?: TParams) => {
        deps.send({ jsonrpc: '2.0', method, params })
    }

    const resolvePendingRequest = (message: JsonRpcResponse) => {
        if (message.id === null) return
        const pendingRequest = pendingRequests.get(message.id)
        if (!pendingRequest) return
        pendingRequests.delete(message.id)
        if (isJsonRpcErrorResponse(message)) {
            pendingRequest.reject((message as JsonRpcErrorResponse).error)
            return
        }
        pendingRequest.resolve(message.result)
    }

    const dispatchNotification = (notification: JsonRpcNotification) => {
        if (notification.method === 'textDocument/publishDiagnostics') {
            const params = notification.params as PublishDiagnosticsParams
            diagnosticsListeners.forEach((listener) => listener(params))
        }
        deps.onNotification(notification)
    }

    /**
     * Registers a handler scoped to *this* client instance for a server→client request method,
     * checked before the process-wide fallback registry (`server-request-handler-registry.ts`).
     * Exists so a request whose correct handling depends on which session/server sent it (e.g.
     * `workspace/applyEdit`, which must only be allowed to touch files under *this session's own*
     * project root — see `workspace-edit-apply-handler.ts`) can be answered with that session's
     * own context instead of a single global handler shared indiscriminately by every session.
     */
    const registerRequestHandler = (method: string, handler: ServerRequestHandler) => {
        instanceRequestHandlers.set(method, handler)
        return () => {
            if (instanceRequestHandlers.get(method) === handler) instanceRequestHandlers.delete(method)
        }
    }

    const handleServerRequest = async (message: JsonRpcRequest) => {
        const handler = instanceRequestHandlers.get(message.method) ?? getServerRequestHandler(message.method)
        if (!handler) {
            deps.send({
                jsonrpc: '2.0',
                id: message.id,
                error: { code: JSON_RPC_ERROR_CODE.METHOD_NOT_FOUND, message: `method not found: ${message.method}` },
            })
            return
        }
        try {
            const result = await handler(message.params)
            deps.send({ jsonrpc: '2.0', id: message.id, result })
        } catch (error) {
            deps.send({
                jsonrpc: '2.0',
                id: message.id,
                error: { code: JSON_RPC_ERROR_CODE.INTERNAL_ERROR, message: error instanceof Error ? error.message : String(error) },
            })
        }
    }

    const handleMessage = (raw: unknown) => {
        if (isJsonRpcResponse(raw)) {
            resolvePendingRequest(raw)
            return
        }
        if (isJsonRpcRequest(raw)) {
            void handleServerRequest(raw)
            return
        }
        if (isJsonRpcNotification(raw)) {
            dispatchNotification(raw)
        }
    }

    const initialize = async <TParams = unknown>(params: TParams) => {
        const result = await request<InitializeResult, TParams>('initialize', params)
        serverCapabilities = result.capabilities
        notify('initialized', {})
        return result
    }

    const getCapabilities = () => serverCapabilities

    /** The client-tracked version for an open document's uri, or `undefined` if it isn't open. Used to reject a `WorkspaceEdit` computed against a since-superseded version (`workspace-edit-applier.ts`). */
    const getDocumentVersion = (uri: string) => documentVersions.get(uri)

    const supports = (predicate: (capabilities: ServerCapabilities) => boolean) => serverCapabilities !== null && predicate(serverCapabilities)

    const onDiagnostics = (listener: (params: PublishDiagnosticsParams) => void) => {
        diagnosticsListeners.add(listener)
        return { dispose: () => diagnosticsListeners.delete(listener) }
    }

    const didOpen = (document: TextDocumentItem) => {
        documentVersions.set(document.uri, document.version)
        notify('textDocument/didOpen', { textDocument: document })
    }

    const didChange = (uri: string, changes: IncrementalTextDocumentContentChangeEvent[]) => {
        const currentVersion = documentVersions.get(uri)
        if (currentVersion === undefined) throw new LspDocumentNotOpenError(uri)
        const nextVersion = currentVersion + 1
        documentVersions.set(uri, nextVersion)
        notify('textDocument/didChange', { textDocument: { uri, version: nextVersion }, contentChanges: changes })
    }

    const didClose = (uri: string) => {
        if (!documentVersions.has(uri)) return
        documentVersions.delete(uri)
        notify('textDocument/didClose', { textDocument: { uri } })
    }

    const didSave = (uri: string) => {
        if (!documentVersions.has(uri)) return
        notify('textDocument/didSave', { textDocument: { uri } })
    }

    const dispose = () => {
        pendingRequests.forEach((pendingRequest) => pendingRequest.reject(new Error('lsp client disposed')))
        pendingRequests.clear()
        diagnosticsListeners.clear()
        documentVersions.clear()
        instanceRequestHandlers.clear()
    }

    return {
        request,
        notify,
        handleMessage,
        initialize,
        getCapabilities,
        getDocumentVersion,
        registerRequestHandler,
        supports,
        onDiagnostics,
        didOpen,
        didChange,
        didClose,
        didSave,
        dispose,
    }
}

export type LspClient = ReturnType<typeof createLspClient>
