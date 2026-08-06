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
import { createRequestIdGenerator, isJsonRpcErrorResponse, isJsonRpcNotification, isJsonRpcResponse } from '@shared/lib/lsp/protocol'

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

type OutgoingMessage = JsonRpcRequest | JsonRpcNotification

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
    'textDocument/diagnostic': (capabilities) => capabilities.diagnosticProvider !== undefined,
}

export const createLspClient = (deps: LspClientDeps) => {
    const nextRequestId = createRequestIdGenerator()
    const pendingRequests = new Map<JsonRpcId, PendingRequest>()
    const diagnosticsListeners = new Set<(params: PublishDiagnosticsParams) => void>()
    const documentVersions = new Map<string, number>()
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

    const handleMessage = (raw: unknown) => {
        if (isJsonRpcResponse(raw)) {
            resolvePendingRequest(raw)
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

    const dispose = () => {
        pendingRequests.forEach((pendingRequest) => pendingRequest.reject(new Error('lsp client disposed')))
        pendingRequests.clear()
        diagnosticsListeners.clear()
        documentVersions.clear()
    }

    return {
        request,
        notify,
        handleMessage,
        initialize,
        getCapabilities,
        supports,
        onDiagnostics,
        didOpen,
        didChange,
        didClose,
        dispose,
    }
}

export type LspClient = ReturnType<typeof createLspClient>
