import type { LspInitializationOptionsValue, LspServerId, ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import type { LspClient, OutgoingMessage } from '@shared/lib/lsp/client'
import { createLspClient } from '@shared/lib/lsp/client'
import { SEMANTIC_TOKEN_MODIFIERS, SEMANTIC_TOKEN_TYPES, SYMBOL_KIND_VALUE_SET } from '@shared/lib/lsp/protocol'
import { registerCodeAction } from '@shared/lib/lsp/adapters/code-action'
import { registerCodeLens } from '@shared/lib/lsp/adapters/code-lens'
import { registerCompletion } from '@shared/lib/lsp/adapters/completion'
import { registerDeclaration } from '@shared/lib/lsp/adapters/declaration'
import { registerDefinition } from '@shared/lib/lsp/adapters/definition'
import { registerDiagnostics } from '@shared/lib/lsp/adapters/diagnostics'
import { registerDocumentHighlight } from '@shared/lib/lsp/adapters/document-highlight'
import { registerDocumentSymbol } from '@shared/lib/lsp/adapters/document-symbol'
import { registerFoldingRange } from '@shared/lib/lsp/adapters/folding-range'
import { registerFormatting, registerOnTypeFormatting, registerRangeFormatting } from '@shared/lib/lsp/adapters/formatting'
import { registerHover } from '@shared/lib/lsp/adapters/hover'
import { registerImplementation } from '@shared/lib/lsp/adapters/implementation'
import { registerInlayHints } from '@shared/lib/lsp/adapters/inlay-hints'
import { registerReferences } from '@shared/lib/lsp/adapters/references'
import { registerRename } from '@shared/lib/lsp/adapters/rename'
import { registerSelectionRange } from '@shared/lib/lsp/adapters/selection-range'
import { registerSemanticTokens, triggerSemanticTokensRefresh } from '@shared/lib/lsp/adapters/semantic-tokens'
import { registerSignatureHelp } from '@shared/lib/lsp/adapters/signature-help'
import { registerTypeDefinition } from '@shared/lib/lsp/adapters/type-definition'
import {
    RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID,
    RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID,
    registerSessionExecuteCommands,
} from '@shared/lib/lsp/command-relay'
import { createWorkspaceApplyEditHandler } from '@shared/lib/lsp/workspace-edit-apply-handler'
import { sendLspMessage, spawnLspSession, stopLspSession } from '@entities/lsp/lsp.ipc'

type Disposable = { dispose: () => void }

const LANGUAGE_ADAPTER_REGISTRARS = [
    registerCompletion,
    registerDeclaration,
    registerDefinition,
    registerDocumentHighlight,
    registerDocumentSymbol,
    registerFoldingRange,
    registerFormatting,
    registerHover,
    registerImplementation,
    registerInlayHints,
    registerOnTypeFormatting,
    registerRangeFormatting,
    registerReferences,
    registerRename,
    registerSelectionRange,
    registerSignatureHelp,
    registerTypeDefinition,
]

export type SessionRecord = {
    refCount: number
    root: string
    ready: Promise<{
        client: LspClient
        sessionId: string
        executeCommandsDisposable: Disposable
        applyEditDisposable: Disposable
        semanticTokensRefreshDisposable: Disposable
    }>
    languageDisposables: Map<string, Disposable[]>
    diagnosticsDisposable: Disposable | null
    openDocuments: Map<string, number>
}

const sessionsByKey = new Map<string, SessionRecord>()
const waitersByKey = new Map<string, Set<() => void>>()
const languageAdapterListeners = new Set<() => void>()

const toSessionKey = (projectId: ProjectId, serverId: LspServerId) => `${projectId}::${serverId}`

const notifyWaiters = (key: string) => {
    const waiters = waitersByKey.get(key)
    if (!waiters) return
    waitersByKey.delete(key)
    waiters.forEach((waiter) => waiter())
}

const toWorkspaceFolderName = (root: string) => root.split('/').filter(Boolean).at(-1) ?? root

const CODE_ACTION_KIND_VALUE_SET = [
    '',
    'quickfix',
    'refactor',
    'refactor.extract',
    'refactor.inline',
    'refactor.rewrite',
    'source',
    'source.organizeImports',
    'source.fixAll',
] as const

const FOLDING_RANGE_LIMIT = 5000

const buildInitializeParams = (root: string, initializationOptions?: LspInitializationOptionsValue | null) => {
    const rootUri = monaco.Uri.file(root).toString()
    const hasInitializationOptions = initializationOptions !== undefined && initializationOptions !== null
    return {
        processId: null,
        clientInfo: { name: 'TAIDE' },
        rootUri,
        rootPath: root,
        workspaceFolders: [{ uri: rootUri, name: toWorkspaceFolderName(root) }],
        ...(hasInitializationOptions ? { initializationOptions } : {}),
        capabilities: {
            general: { positionEncodings: ['utf-16'] },
            /**
             * rust-analyzer only emits `HasImpls`/`HasReferences`/runnable CodeLenses whose click
             * commands it advertises here (`experimental.commands.commands`) — without this, ra
             * silently produces zero CodeLenses (contract §3.4). `runSingle`/`debugSingle` stay
             * unlisted (no run/debug infra yet), so ra's run/debug lenses stay off by design.
             */
            experimental: { commands: { commands: [RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID, RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID] } },
            workspace: {
                workspaceFolders: true,
                configuration: true,
                applyEdit: true,
                executeCommand: {},
                workspaceEdit: {
                    documentChanges: true,
                    resourceOperations: ['create', 'rename', 'delete'],
                    /**
                     * `applyWorkspaceEdit` (workspace-edit-applier.ts) stops at the first failed
                     * operation and does not roll back ones already applied — exactly LSP 3.17's
                     * `abort` semantics ("operations executed before the failing one stay
                     * executed"), not `textOnlyTransactional`'s stronger all-or-nothing guarantee
                     * for text-only edits. Declaring the latter would promise a rollback this
                     * client cannot perform.
                     */
                    failureHandling: 'abort',
                },
                codeLens: { refreshSupport: true },
                /**
                 * Advertises support for `workspace/semanticTokens/refresh` (Wave F contract §3.1).
                 * Unlike `codeLens.refreshSupport` above — whose `workspace/codeLens/refresh` handler
                 * is registered once, process-wide, in `server-request-handler-registry.ts` — the
                 * semantic tokens refresh handler must be registered *per session* via
                 * `client.registerRequestHandler` (the `workspace/applyEdit` pattern already used in
                 * `createSession` below), so a refresh request from one server only invalidates that
                 * server's own semantic tokens adapter instead of every open session's. That handler
                 * is wired in the same commit that registers the semantic tokens adapter itself
                 * (Wave F Phase D) — declaring this capability ahead of that handler would make the
                 * server's eventual refresh request bounce off as an unhandled `-32601
                 * MethodNotFound`, so the two must land together.
                 */
                semanticTokens: { refreshSupport: true },
                /**
                 * Enables `⌘T` Workspace Symbol search (contract §3.2, `adapters/workspace-symbol.ts`).
                 * `symbolKind.valueSet` mirrors `textDocument.documentSymbol` below so a server never
                 * has to guess which `SymbolKind` numbers this client understands for either request.
                 */
                symbol: { symbolKind: { valueSet: SYMBOL_KIND_VALUE_SET } },
            },
            textDocument: {
                synchronization: { dynamicRegistration: false, didSave: true },
                completion: { completionItem: { snippetSupport: true }, contextSupport: true },
                hover: { contentFormat: ['markdown', 'plaintext'] },
                signatureHelp: {},
                definition: { linkSupport: true },
                references: {},
                /**
                 * `hierarchicalDocumentSymbolSupport` tells the server it may return nested
                 * `DocumentSymbol` trees instead of flat `SymbolInformation[]` — the palette's `@`
                 * mode (`command-palette.tsx`) and the Breadcrumbs symbol path (Wave D §3.3) both
                 * depend on that hierarchy to build a "Class > method" container label.
                 */
                documentSymbol: { hierarchicalDocumentSymbolSupport: true, symbolKind: { valueSet: SYMBOL_KIND_VALUE_SET } },
                formatting: {},
                rangeFormatting: {},
                onTypeFormatting: {},
                rename: { prepareSupport: true },
                publishDiagnostics: { relatedInformation: true },
                inlayHint: {},
                /**
                 * `requests.full.delta: true` opts into `SemanticTokensDelta` responses so a server
                 * may send only the edited spans instead of the whole token stream on every edit
                 * (Wave F contract §3.1/§3.3 decision ②) — the adapter re-encodes whatever comes back
                 * (full or delta-applied) into a single monaco-facing `SemanticTokens` result, so no
                 * monaco-side delta plumbing is needed here. `range` support is intentionally not
                 * requested (1st cut excludes viewport-scoped requests, contract §4). `tokenTypes`/
                 * `tokenModifiers` mirror the client's own legend (`SEMANTIC_TOKEN_TYPES`/
                 * `SEMANTIC_TOKEN_MODIFIERS`) so a server can freely emit indices into either list —
                 * the adapter maps the server's *actual* advertised legend (which may use a different
                 * index order or extra non-standard names, e.g. rust-analyzer) back onto these names,
                 * dropping anything it can't map (contract §2-1 washout defense). `augmentsSyntaxTokens:
                 * true` (not `augmentsTokens` — an easy typo, since this token pairs with syntax
                 * highlighting rather than replacing it) tells the server this client keeps its base
                 * syntax colors and only layers semantic colors on top.
                 */
                semanticTokens: {
                    requests: { full: { delta: true } },
                    tokenTypes: [...SEMANTIC_TOKEN_TYPES],
                    tokenModifiers: [...SEMANTIC_TOKEN_MODIFIERS],
                    formats: ['relative'],
                    overlappingTokenSupport: false,
                    multilineTokenSupport: false,
                    serverCancelSupport: false,
                    augmentsSyntaxTokens: true,
                },
                codeAction: {
                    codeActionLiteralSupport: { codeActionKind: { valueSet: CODE_ACTION_KIND_VALUE_SET } },
                    isPreferredSupport: true,
                    disabledSupport: true,
                    dataSupport: true,
                    resolveSupport: { properties: ['edit'] },
                },
                codeLens: {},
                foldingRange: { lineFoldingOnly: true, rangeLimit: FOLDING_RANGE_LIMIT },
                implementation: { linkSupport: true },
                typeDefinition: { linkSupport: true },
                declaration: { linkSupport: true },
            },
        },
    }
}

const createSession = async (
    projectId: ProjectId,
    serverId: LspServerId,
    root: string,
    initializationOptions?: LspInitializationOptionsValue | null,
) => {
    let sessionId: string | null = null
    const pendingOutgoingMessages: OutgoingMessage[] = []

    const client = createLspClient({
        send: (message) => {
            if (!sessionId) {
                pendingOutgoingMessages.push(message)
                return
            }
            void sendLspMessage({ sessionId, message: JSON.stringify(message) }).catch(() => undefined)
        },
        onNotification: () => undefined,
    })

    sessionId = await spawnLspSession({
        projectId,
        serverId,
        root,
        onMessage: (raw) => {
            try {
                client.handleMessage(JSON.parse(raw))
            } catch {
                return
            }
        },
    })

    for (const message of pendingOutgoingMessages) {
        void sendLspMessage({ sessionId, message: JSON.stringify(message) }).catch(() => undefined)
    }
    pendingOutgoingMessages.length = 0

    await client.initialize(buildInitializeParams(root, initializationOptions))
    const executeCommandsDisposable = registerSessionExecuteCommands(monaco, client, client.getCapabilities()?.executeCommandProvider?.commands)
    /**
     * Root-scoped per-session `workspace/applyEdit` handler (see `workspace-edit-apply-handler.ts`)
     * — registered on this session's own `client` instance so it always wins over the unscoped
     * process-wide fallback (`registerWorkspaceApplyEditHandler`, app bootstrap), and so this
     * session's server can never be asked to believe it edited files under a different project.
     */
    const applyEditDisposable = {
        dispose: client.registerRequestHandler('workspace/applyEdit', createWorkspaceApplyEditHandler(monaco, root, client, projectId)),
    }
    /**
     * Session-scoped `workspace/semanticTokens/refresh` handler (contract §3.1, promised by the
     * `semanticTokens.refreshSupport` capability declared in `buildInitializeParams` above) — mirrors
     * `applyEditDisposable`'s per-client registration pattern so a refresh push from this session's
     * server only fires this session's own `registerSemanticTokens` adapters (`onDidChange`), never
     * every open session's.
     */
    const semanticTokensRefreshDisposable = {
        dispose: client.registerRequestHandler('workspace/semanticTokens/refresh', async () => {
            triggerSemanticTokensRefresh(client)
            return null
        }),
    }

    return { client, sessionId, executeCommandsDisposable, applyEditDisposable, semanticTokensRefreshDisposable }
}

const disposeSession = async (key: string, record: SessionRecord) => {
    const session = await record.ready.catch(() => null)
    for (const disposables of record.languageDisposables.values()) {
        for (const disposable of disposables) disposable.dispose()
    }
    record.diagnosticsDisposable?.dispose()
    if (!session) return
    session.executeCommandsDisposable.dispose()
    session.applyEditDisposable.dispose()
    session.semanticTokensRefreshDisposable.dispose()
    session.client.dispose()
    await stopLspSession(session.sessionId, record.root).catch(() => undefined)
    if (sessionsByKey.get(key) === record) sessionsByKey.delete(key)
}

export const acquireLspSession = (
    projectId: ProjectId,
    serverId: LspServerId,
    root: string,
    initializationOptions?: LspInitializationOptionsValue | null,
) => {
    const key = toSessionKey(projectId, serverId)
    const existing = sessionsByKey.get(key)
    if (existing) {
        existing.refCount += 1
        return { key, record: existing }
    }

    const record: SessionRecord = {
        refCount: 1,
        root,
        ready: createSession(projectId, serverId, root, initializationOptions),
        languageDisposables: new Map(),
        diagnosticsDisposable: null,
        openDocuments: new Map(),
    }
    void record.ready.catch(() => sessionsByKey.delete(key))
    sessionsByKey.set(key, record)
    notifyWaiters(key)
    return { key, record }
}

export const peekLspSession = (projectId: ProjectId, serverId: LspServerId) => sessionsByKey.get(toSessionKey(projectId, serverId)) ?? null

/**
 * Every currently-acquired session for `projectId`, across every server/language — used by the
 * command palette's `⌘T` Workspace Symbol search (`command-palette.tsx`), which queries all of a
 * project's active sessions in parallel rather than one language's sessions like `waitForLspSession`
 * callers (outline, `@` mode) do.
 */
export const listSessionRecordsForProject = (projectId: ProjectId): SessionRecord[] => {
    const prefix = `${projectId}::`
    return Array.from(sessionsByKey.entries())
        .filter(([key]) => key.startsWith(prefix))
        .map(([, record]) => record)
}

export const waitForLspSession = (projectId: ProjectId, serverId: LspServerId) => {
    const key = toSessionKey(projectId, serverId)
    const existing = sessionsByKey.get(key)
    if (existing) return { promise: Promise.resolve<SessionRecord | null>(existing), cancel: () => {} }

    let waiter: () => void = () => {}
    const promise = new Promise<SessionRecord | null>((resolve) => {
        waiter = () => resolve(sessionsByKey.get(key) ?? null)
        const waiters = waitersByKey.get(key) ?? new Set()
        waiters.add(waiter)
        waitersByKey.set(key, waiters)
    })
    const cancel = () => waitersByKey.get(key)?.delete(waiter)
    return { promise, cancel }
}

export const releaseLspSession = (key: string, record: SessionRecord) => {
    record.refCount -= 1
    if (record.refCount > 0) return
    if (sessionsByKey.get(key) === record) sessionsByKey.delete(key)
    void disposeSession(key, record)
}

/**
 * Notifies subscribers whenever LSP-backed monaco providers (formatting/rename/etc.) are newly
 * registered for a language. `editor.getSupportedActions()` precondition results depend on these
 * providers, so consumers that cache a supported-action snapshot must recompute it on this event.
 */
export const subscribeLanguageAdapterRegistration = (listener: () => void) => {
    languageAdapterListeners.add(listener)
    return () => {
        languageAdapterListeners.delete(listener)
    }
}

export const ensureLanguageRegistered = (
    record: SessionRecord,
    client: LspClient,
    serverId: LspServerId,
    languageId: string,
    isCodeLensEnabled: () => boolean = () => true,
    isSemanticHighlightingEnabled: () => boolean = () => true,
) => {
    if (!record.diagnosticsDisposable) record.diagnosticsDisposable = registerDiagnostics(monaco, client, serverId)
    if (record.languageDisposables.has(languageId)) return

    const disposables = [
        ...LANGUAGE_ADAPTER_REGISTRARS.map((register) => register(monaco, client, languageId)),
        registerCodeAction(monaco, client, serverId, languageId),
        registerCodeLens(monaco, client, languageId, isCodeLensEnabled),
        registerSemanticTokens(monaco, client, languageId, isSemanticHighlightingEnabled),
    ]
    record.languageDisposables.set(languageId, disposables)
    for (const listener of languageAdapterListeners) listener()
}

export const acquireDocument = (record: SessionRecord, client: LspClient, uri: string, languageId: string, text: string) => {
    const current = record.openDocuments.get(uri) ?? 0
    record.openDocuments.set(uri, current + 1)
    if (current === 0) client.didOpen({ uri, languageId, version: 0, text })
}

export const releaseDocument = (record: SessionRecord, client: LspClient, uri: string) => {
    const current = record.openDocuments.get(uri)
    if (!current) return
    if (current <= 1) {
        record.openDocuments.delete(uri)
        client.didClose(uri)
        return
    }
    record.openDocuments.set(uri, current - 1)
}
