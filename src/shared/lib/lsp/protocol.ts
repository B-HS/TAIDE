export type JsonRpcId = number | string

type JsonRpcEnvelope = { jsonrpc: '2.0' }

export type JsonRpcRequest<TParams = unknown> = JsonRpcEnvelope & {
    id: JsonRpcId
    method: string
    params?: TParams
}

export type JsonRpcNotification<TParams = unknown> = JsonRpcEnvelope & {
    method: string
    params?: TParams
}

export type JsonRpcResponseError = {
    code: number
    message: string
    data?: unknown
}

export type JsonRpcSuccessResponse<TResult = unknown> = JsonRpcEnvelope & {
    id: JsonRpcId
    result: TResult
}

export type JsonRpcErrorResponse = JsonRpcEnvelope & {
    id: JsonRpcId | null
    error: JsonRpcResponseError
}

export type JsonRpcResponse<TResult = unknown> = JsonRpcSuccessResponse<TResult> | JsonRpcErrorResponse

export type JsonRpcMessage = JsonRpcRequest | JsonRpcNotification | JsonRpcResponse

const isPlainObject = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null

const isJsonRpcId = (value: unknown): value is JsonRpcId => typeof value === 'number' || typeof value === 'string'

export const isJsonRpcResponse = (message: unknown): message is JsonRpcResponse =>
    isPlainObject(message) && 'id' in message && (message.id === null || isJsonRpcId(message.id)) && ('result' in message || 'error' in message)

export const isJsonRpcErrorResponse = (message: JsonRpcResponse): message is JsonRpcErrorResponse => 'error' in message

export const isJsonRpcNotification = (message: unknown): message is JsonRpcNotification =>
    isPlainObject(message) && typeof message.method === 'string' && !('id' in message)

export const isJsonRpcRequest = (message: unknown): message is JsonRpcRequest =>
    isPlainObject(message) && typeof message.method === 'string' && 'id' in message && isJsonRpcId((message as { id: unknown }).id)

export const createRequestIdGenerator = () => {
    let nextId = 0
    return () => {
        nextId += 1
        return nextId
    }
}

export const isCapabilityEnabled = (capability: boolean | object | undefined) => capability !== undefined && capability !== false

export type LspPosition = { line: number; character: number }

export type LspRange = { start: LspPosition; end: LspPosition }

export type TextDocumentIdentifier = { uri: string }

export type VersionedTextDocumentIdentifier = TextDocumentIdentifier & { version: number }

export type TextDocumentItem = { uri: string; languageId: string; version: number; text: string }

export type IncrementalTextDocumentContentChangeEvent = { range: LspRange; rangeLength?: number; text: string }

export const DIAGNOSTIC_SEVERITY = { ERROR: 1, WARNING: 2, INFORMATION: 3, HINT: 4 } as const
export type DiagnosticSeverity = (typeof DIAGNOSTIC_SEVERITY)[keyof typeof DIAGNOSTIC_SEVERITY]

export type Diagnostic = {
    range: LspRange
    severity?: DiagnosticSeverity
    code?: string | number
    source?: string
    message: string
    tags?: number[]
}

export type PublishDiagnosticsParams = { uri: string; version?: number; diagnostics: Diagnostic[] }

export type MarkupContent = { kind: 'plaintext' | 'markdown'; value: string }

export const markupContentToString = (content: string | MarkupContent | undefined): string | undefined =>
    typeof content === 'string' || content === undefined ? content : content.value

export type TextEdit = { range: LspRange; newText: string }

export type CompletionItem = {
    label: string
    kind?: number
    detail?: string
    documentation?: string | MarkupContent
    insertText?: string
    insertTextFormat?: number
    sortText?: string
    filterText?: string
    textEdit?: TextEdit
    additionalTextEdits?: TextEdit[]
}

export type CompletionList = { isIncomplete: boolean; items: CompletionItem[] }

export type Hover = { contents: string | MarkupContent | (string | MarkupContent)[]; range?: LspRange }

export type Location = { uri: string; range: LspRange }

export type LocationLink = { targetUri: string; targetRange: LspRange; targetSelectionRange: LspRange; originSelectionRange?: LspRange }

export type WorkspaceEdit = { changes?: Record<string, TextEdit[]> }

export type PrepareRenameResult = LspRange | { range: LspRange; placeholder: string }

export type SignatureInformation = {
    label: string
    documentation?: string | MarkupContent
    parameters?: { label: string | [number, number]; documentation?: string | MarkupContent }[]
}

export type SignatureHelp = { signatures: SignatureInformation[]; activeSignature?: number; activeParameter?: number }

export type InlayHintLabelPart = { value: string; tooltip?: string | MarkupContent }

export type InlayHint = {
    position: LspPosition
    label: string | InlayHintLabelPart[]
    kind?: number
    tooltip?: string | MarkupContent
    paddingLeft?: boolean
    paddingRight?: boolean
}

export type DocumentSymbol = {
    name: string
    detail?: string
    kind: number
    range: LspRange
    selectionRange: LspRange
    children?: DocumentSymbol[]
}

export type SymbolInformation = { name: string; kind: number; location: Location }

export type ServerCapabilities = {
    positionEncoding?: string
    completionProvider?: { triggerCharacters?: string[]; resolveProvider?: boolean }
    hoverProvider?: boolean | Record<string, never>
    definitionProvider?: boolean | Record<string, never>
    referencesProvider?: boolean | Record<string, never>
    renameProvider?: boolean | { prepareProvider?: boolean }
    documentFormattingProvider?: boolean | Record<string, never>
    documentRangeFormattingProvider?: boolean | Record<string, never>
    signatureHelpProvider?: { triggerCharacters?: string[]; retriggerCharacters?: string[] }
    inlayHintProvider?: boolean | Record<string, never>
    documentSymbolProvider?: boolean | Record<string, never>
    diagnosticProvider?: { interFileDependencies?: boolean; workspaceDiagnostics?: boolean }
}

export type InitializeResult = {
    capabilities: ServerCapabilities
    serverInfo?: { name: string; version?: string }
}
