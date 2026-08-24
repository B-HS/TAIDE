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

export const JSON_RPC_ERROR_CODE = { METHOD_NOT_FOUND: -32601, INTERNAL_ERROR: -32603 } as const

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
    data?: unknown
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

export type OptionalVersionedTextDocumentIdentifier = TextDocumentIdentifier & { version: number | null }

export type TextDocumentEdit = { textDocument: OptionalVersionedTextDocumentIdentifier; edits: TextEdit[] }

export type CreateFileOptions = { overwrite?: boolean; ignoreIfExists?: boolean }
export type CreateFile = { kind: 'create'; uri: string; options?: CreateFileOptions }

export type RenameFileOptions = { overwrite?: boolean; ignoreIfExists?: boolean }
export type RenameFile = { kind: 'rename'; oldUri: string; newUri: string; options?: RenameFileOptions }

export type DeleteFileOptions = { recursive?: boolean; ignoreIfNotExists?: boolean }
export type DeleteFile = { kind: 'delete'; uri: string; options?: DeleteFileOptions }

export type DocumentChangeOperation = TextDocumentEdit | CreateFile | RenameFile | DeleteFile

export type WorkspaceEdit = { changes?: Record<string, TextEdit[]>; documentChanges?: DocumentChangeOperation[] }

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

export type ResolvableWorkspaceSymbolLocation = Location | { uri: string }

export type WorkspaceSymbol = {
    name: string
    kind: number
    tags?: number[]
    containerName?: string
    location: ResolvableWorkspaceSymbolLocation
    data?: unknown
}

export const SYMBOL_KIND = {
    FILE: 1,
    MODULE: 2,
    NAMESPACE: 3,
    PACKAGE: 4,
    CLASS: 5,
    METHOD: 6,
    PROPERTY: 7,
    FIELD: 8,
    CONSTRUCTOR: 9,
    ENUM: 10,
    INTERFACE: 11,
    FUNCTION: 12,
    VARIABLE: 13,
    CONSTANT: 14,
    STRING: 15,
    NUMBER: 16,
    BOOLEAN: 17,
    ARRAY: 18,
    OBJECT: 19,
    KEY: 20,
    NULL: 21,
    ENUM_MEMBER: 22,
    STRUCT: 23,
    EVENT: 24,
    OPERATOR: 25,
    TYPE_PARAMETER: 26,
} as const
export type SymbolKind = (typeof SYMBOL_KIND)[keyof typeof SYMBOL_KIND]

/** Full LSP `SymbolKind` enum range, declared as the `symbolKind.valueSet` client capability for both `textDocument/documentSymbol` and `workspace/symbol` (`initialize-params.ts`'s `buildInitializeParams`) so a server never has to guess which kinds this client understands. */
export const SYMBOL_KIND_VALUE_SET: SymbolKind[] = Object.values(SYMBOL_KIND)

export const DOCUMENT_HIGHLIGHT_KIND = { TEXT: 1, READ: 2, WRITE: 3 } as const
export type DocumentHighlightKind = (typeof DOCUMENT_HIGHLIGHT_KIND)[keyof typeof DOCUMENT_HIGHLIGHT_KIND]

export type DocumentHighlight = { range: LspRange; kind?: DocumentHighlightKind }

export type SelectionRange = { range: LspRange; parent?: SelectionRange }

export const SEMANTIC_TOKEN_TYPES = [
    'namespace',
    'type',
    'class',
    'enum',
    'interface',
    'struct',
    'typeParameter',
    'parameter',
    'variable',
    'property',
    'enumMember',
    'event',
    'function',
    'method',
    'macro',
    'keyword',
    'modifier',
    'comment',
    'string',
    'number',
    'regexp',
    'operator',
    'decorator',
] as const
export type SemanticTokenType = (typeof SEMANTIC_TOKEN_TYPES)[number]

export const SEMANTIC_TOKEN_MODIFIERS = [
    'declaration',
    'definition',
    'readonly',
    'static',
    'deprecated',
    'abstract',
    'async',
    'modification',
    'documentation',
    'defaultLibrary',
] as const
export type SemanticTokenModifier = (typeof SEMANTIC_TOKEN_MODIFIERS)[number]

export type SemanticTokensLegend = { tokenTypes: string[]; tokenModifiers: string[] }
export type SemanticTokensOptions = { legend: SemanticTokensLegend; full?: boolean | { delta?: boolean }; range?: boolean | Record<string, never> }
export type DocumentOnTypeFormattingOptions = { firstTriggerCharacter: string; moreTriggerCharacter?: string[] }

export type SemanticTokens = { resultId?: string; data: number[] }
export type SemanticTokensEdit = { start: number; deleteCount: number; data?: number[] }
export type SemanticTokensDelta = { resultId?: string; edits: SemanticTokensEdit[] }

export type CodeActionOptions = { codeActionKinds?: string[]; resolveProvider?: boolean }
export type CodeLensOptions = { resolveProvider?: boolean }
export type ExecuteCommandOptions = { commands?: string[] }

export type ServerCapabilities = {
    positionEncoding?: string
    completionProvider?: { triggerCharacters?: string[]; resolveProvider?: boolean }
    hoverProvider?: boolean | Record<string, never>
    definitionProvider?: boolean | Record<string, never>
    referencesProvider?: boolean | Record<string, never>
    renameProvider?: boolean | { prepareProvider?: boolean }
    documentFormattingProvider?: boolean | Record<string, never>
    documentRangeFormattingProvider?: boolean | Record<string, never>
    documentOnTypeFormattingProvider?: DocumentOnTypeFormattingOptions
    signatureHelpProvider?: { triggerCharacters?: string[]; retriggerCharacters?: string[] }
    inlayHintProvider?: boolean | Record<string, never>
    documentSymbolProvider?: boolean | Record<string, never>
    workspaceSymbolProvider?: boolean | { resolveProvider?: boolean }
    documentHighlightProvider?: boolean | Record<string, never>
    selectionRangeProvider?: boolean | Record<string, never>
    semanticTokensProvider?: SemanticTokensOptions
    diagnosticProvider?: { interFileDependencies?: boolean; workspaceDiagnostics?: boolean }
    codeActionProvider?: boolean | CodeActionOptions
    codeLensProvider?: CodeLensOptions
    foldingRangeProvider?: boolean | Record<string, never>
    implementationProvider?: boolean | Record<string, never>
    typeDefinitionProvider?: boolean | Record<string, never>
    declarationProvider?: boolean | Record<string, never>
    executeCommandProvider?: ExecuteCommandOptions
}

export type InitializeResult = {
    capabilities: ServerCapabilities
    serverInfo?: { name: string; version?: string }
}
