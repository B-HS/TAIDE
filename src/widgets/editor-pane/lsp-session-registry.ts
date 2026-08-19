import type { LspInitializationOptionsValue, LspServerId, LspSessionStatusChanged, ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import type { LspClient, OutgoingMessage } from '@shared/lib/lsp/client'
import { createLspClient } from '@shared/lib/lsp/client'
import { SEMANTIC_TOKEN_MODIFIERS, SEMANTIC_TOKEN_TYPES, SYMBOL_KIND_VALUE_SET } from '@shared/lib/lsp/protocol'
import { registerCodeAction } from '@shared/lib/lsp/adapters/code-action'
import { registerCodeLens, triggerCodeLensRefresh } from '@shared/lib/lsp/adapters/code-lens'
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
import { confirmLspReinitialize, sendLspMessage, spawnLspSession, stopLspSession } from '@entities/lsp/lsp.ipc'
import { registerLspSessionAllFlush, registerLspSessionProjectFlush } from '@entities/lsp/lsp-session-flush-registry'

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

type ConnectionState = {
    languageDisposables: Map<string, Disposable[]>
    diagnosticsDisposable: Disposable | null
    openDocuments: Map<string, number>
}

const createConnectionState = (): ConnectionState => ({
    languageDisposables: new Map(),
    diagnosticsDisposable: null,
    openDocuments: new Map(),
})

type ResolvedSession = {
    client: LspClient
    sessionId: string
    executeCommandsDisposable: Disposable
    applyEditDisposable: Disposable
    semanticTokensRefreshDisposable: Disposable
    codeLensRefreshDisposable: Disposable
}

/**
 * The state genuinely shared by every root a `sharesSessions` server's single connection currently
 * serves (R7#7) — one real LSP process, one `LspClient`, one set of monaco provider registrations.
 * `roots` mirrors Rust's `SessionEntry.roots` refcount-by-root Vec (minus the count: this side never
 * calls `lsp_spawn` twice for the same root, since a repeat `acquireLspSession` for an
 * already-tracked root is a `sessionsByKey` cache hit that never reaches `createSession` again).
 * `lastObservedGeneration` mirrors `LspSessionStatusChanged.generation` (see that field's doc on
 * `domain::lsp::commands::SessionEntry`) so the module-level status listener only reacts to an
 * actual increase, never a duplicate/out-of-order delivery of an already-handled one.
 */
type SessionGroup = {
    projectId: ProjectId
    serverId: LspServerId
    initializationOptions: LspInitializationOptionsValue | null | undefined
    refCount: number
    disposeTimer: ReturnType<typeof setTimeout> | null
    roots: Set<string>
    state: ConnectionState
    lastObservedGeneration: number
}

/**
 * One `(projectId, serverId, root)` acquisition's handle. `ready` is this specific handle's own
 * promise — for a root that joins an existing `sharesSessions` connection it still resolves to the
 * *identical* {@link ResolvedSession} object the joined session's own handle resolves to (JS promise
 * resolution adopts a thenable's value by reference, not by copy), so mutating a field on it (the
 * reinitialize flow's `executeCommandsDisposable` swap) is visible through every handle sharing it.
 * `group` starts as this handle's own (empty) group and is reassigned in place to the joined
 * session's group the moment a join is discovered (`createSession`) — every other field
 * (`refCount`/`roots`/`state`/disposal timing) lives on `group`, not here, specifically so that
 * reassignment is all it takes for two previously-independent handles to become one shared unit.
 */
export type SessionRecord = {
    group: SessionGroup
    ready: Promise<ResolvedSession>
}

const sessionsByKey = new Map<string, SessionRecord>()
const recordsBySessionId = new Map<string, SessionRecord>()
const waitersByKey = new Map<string, Set<() => void>>()
const languageAdapterListeners = new Set<() => void>()

const toSessionKey = (projectId: ProjectId, serverId: LspServerId, root: string) => `${projectId}::${serverId}::${root}`
const toWaiterKey = (projectId: ProjectId, serverId: LspServerId) => `${projectId}::${serverId}`

/**
 * The first (oldest-inserted) handle currently tracked for `(projectId, serverId)`, regardless of
 * which root it was acquired for — used both by the root-agnostic public API
 * ({@link peekLspSession}/{@link waitForLspSession}/{@link listSessionRecordsForProject}, none of
 * which know or care which specific root a caller's file resolved to) and internally by
 * `acquireLspSession` to find the *sibling* a new root should attempt to join. Always the true
 * fresh/canonical handle for its connection when one exists: the very first root ever acquired for
 * a `(projectId, serverId)` pair can never itself be "joining" anything (nothing existed yet when it
 * was created), and `Map` iteration is insertion-ordered, so the earliest surviving key is always
 * that original handle (or, if it since failed and was evicted, whichever later handle became the
 * new earliest one — self-healing, not stale).
 */
const findAnyRecordForServer = (projectId: ProjectId, serverId: LspServerId): SessionRecord | null => {
    const prefix = `${projectId}::${serverId}::`
    for (const [key, record] of sessionsByKey) {
        if (key.startsWith(prefix)) return record
    }
    return null
}

const notifyWaiters = (waiterKey: string) => {
    const waiters = waitersByKey.get(waiterKey)
    if (!waiters) return
    waitersByKey.delete(waiterKey)
    waiters.forEach((waiter) => waiter())
}

const toWorkspaceFolderName = (root: string) => root.split('/').filter(Boolean).at(-1) ?? root

/**
 * How long a fully-released LSP session (`refCount` reaching 0) stays alive before its process is
 * actually torn down. Covers the common case of a user clicking through several files of the same
 * project/language within a few seconds (file-tree browsing, code review, `⌘P` quick-open chains) —
 * without this, every such switch tore the session down and respawned it, and the teardown
 * (`lsp_stop`) held the shared mutation lock other layout/file/lsp commands queue behind, making
 * the next file open feel briefly frozen (qa contract `docs/acknowledge/2026-08-18-hand-qa-fix-
 * contract.md` §1 item 4). Long enough to absorb that browsing rhythm, short enough that a session
 * nobody reacquires still gets reclaimed soon after the last tab referencing it actually closes.
 */
export const LSP_SESSION_DISPOSE_GRACE_MS = 5000

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

/**
 * `roots` is every workspace root this connection currently serves — for a `sharesSessions` server
 * joined by more than one root (R7#7), `workspaceFolders` advertises all of them up front (matching
 * what `lsp_spawn`'s own `workspace/didChangeWorkspaceFolders` notification keeps in sync for roots
 * added *after* this initial handshake). The first root (insertion order) is used for the legacy
 * singular `rootUri`/`rootPath` fields LSP 3.17 deprecated in favor of `workspaceFolders` but many
 * servers still read.
 */
const buildInitializeParams = (roots: ReadonlySet<string>, initializationOptions?: LspInitializationOptionsValue | null) => {
    const rootList = Array.from(roots)
    const primaryRoot = rootList[0] ?? ''
    const primaryRootUri = monaco.Uri.file(primaryRoot).toString()
    const hasInitializationOptions = initializationOptions !== undefined && initializationOptions !== null
    return {
        processId: null,
        clientInfo: { name: 'TAIDE' },
        rootUri: primaryRootUri,
        rootPath: primaryRoot,
        workspaceFolders: rootList.map((root) => ({ uri: monaco.Uri.file(root).toString(), name: toWorkspaceFolderName(root) })),
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
                 * is likewise registered *per session* on `client` below (F7#4; it used to be a
                 * single process-wide handler in `server-request-handler-registry.ts`, which fired
                 * every open session's listeners on a refresh push from any one server) — the
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

/**
 * Builds (or, for a `sharesSessions` server whose connection `sibling` already owns, joins) one
 * `(projectId, serverId, root)` acquisition's session. `record` is this acquisition's own handle
 * (already inserted into `sessionsByKey`, `group` starting as this handle's own private, empty
 * group); `sibling` — the first other handle already tracked for this `(projectId, serverId)` pair,
 * if any — is awaited to full settlement *before* this function ever calls `spawnLspSession`, so a
 * join attempt can never race `sibling`'s own in-flight `initialize()` handshake (racing it would
 * risk sending a second `initialize` to an already-initializing connection — see the `channels`
 * field doc on Rust's `domain::lsp::commands::SessionEntry` for why that corrupts the connection for
 * every window sharing it).
 *
 * Always builds a full (never-initialized) throwaway `LspClient` first and only *after*
 * `spawnLspSession` resolves checks whether the returned `sessionId` already has a live connection
 * registered (`recordsBySessionId`) — the true, backend-confirmed answer to "does this root's
 * server actually share a connection with `sibling`'s" (`service::should_reuse_session`), which the
 * frontend cannot know in advance (`LspServerDetection` never exposes `sharesSessions`). A join
 * discards the throwaway client (it was never `initialize`d, so nothing is lost) and re-points
 * `record.group` at the already-live one instead — this, not a second `initialize()` call, is what
 * "joining" means on this side; see the `SessionRecord.ready` field doc for how that reassignment
 * makes every field two joined handles need to share (`refCount`/`roots`/`state`) actually shared.
 */
const createSession = async (
    record: SessionRecord,
    sibling: SessionRecord | null,
    projectId: ProjectId,
    serverId: LspServerId,
    root: string,
    initializationOptions?: LspInitializationOptionsValue | null,
): Promise<ResolvedSession> => {
    if (sibling) await sibling.ready.catch(() => undefined)

    let sessionId: string | null = null
    const pendingOutgoingMessages: OutgoingMessage[] = []
    let activeClient: LspClient

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
    activeClient = client

    /**
     * Registered on `client` immediately after it's created — *before* `spawnLspSession` even
     * starts the server process, let alone before the `initialize` round-trip below resolves —
     * so there is no window in which an inbound `workspace/applyEdit` could arrive unhandled by
     * this session's own root-scoped handler. `client.ts`'s `handleServerRequest` checks this
     * instance-level registry before the process-wide fallback registry
     * (`server-request-handler-registry.ts`), and there is deliberately no fallback registered for
     * this method there at all (see `workspace-edit-apply-handler.ts`'s `createWorkspaceApplyEditHandler`
     * doc comment) — a request that outran this registration would otherwise let any session's
     * server believe it could edit files under *any* open project, not just its own. Passes
     * `record.group.roots` *by reference* (not a snapshot) — if this ends up being a fresh session
     * whose group later gains more joined roots, or a throwaway that gets discarded on the join
     * branch below, either way the live set stays correct without re-registering this handler.
     */
    const applyEditDisposable = {
        dispose: client.registerRequestHandler('workspace/applyEdit', createWorkspaceApplyEditHandler(monaco, record.group.roots, client, projectId)),
    }

    sessionId = await spawnLspSession({
        projectId,
        serverId,
        root,
        onMessage: (raw) => {
            try {
                activeClient.handleMessage(JSON.parse(raw))
            } catch {
                return
            }
        },
    })

    const joined = recordsBySessionId.get(sessionId)
    if (joined) {
        applyEditDisposable.dispose()
        client.dispose()
        if (joined.group.disposeTimer !== null) {
            clearTimeout(joined.group.disposeTimer)
            joined.group.disposeTimer = null
        }
        joined.group.refCount += record.group.refCount
        joined.group.roots.add(root)
        record.group = joined.group
        activeClient = (await joined.ready).client
        return joined.ready
    }

    for (const message of pendingOutgoingMessages) {
        void sendLspMessage({ sessionId, message: JSON.stringify(message) }).catch(() => undefined)
    }
    pendingOutgoingMessages.length = 0

    record.group.roots.add(root)
    await client.initialize(buildInitializeParams(record.group.roots, initializationOptions))
    const executeCommandsDisposable = registerSessionExecuteCommands(monaco, client, client.getCapabilities()?.executeCommandProvider?.commands)
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
    /**
     * Session-scoped `workspace/codeLens/refresh` handler (F7#4) — same pattern and same reason as
     * `semanticTokensRefreshDisposable` above, replacing what used to be a single process-wide
     * handler in `server-request-handler-registry.ts` that fired *every* open session's listeners
     * on a refresh push from any one server (two unrelated projects' sessions each recomputed
     * lenses whenever the other's server asked for a refresh).
     */
    const codeLensRefreshDisposable = {
        dispose: client.registerRequestHandler('workspace/codeLens/refresh', async () => {
            triggerCodeLensRefresh(client)
            return null
        }),
    }

    const session: ResolvedSession = {
        client,
        sessionId,
        executeCommandsDisposable,
        applyEditDisposable,
        semanticTokensRefreshDisposable,
        codeLensRefreshDisposable,
    }
    recordsBySessionId.set(sessionId, record)
    return session
}

const disposeSession = async (record: SessionRecord, group: SessionGroup) => {
    const session = await record.ready.catch(() => null)
    for (const disposables of group.state.languageDisposables.values()) {
        for (const disposable of disposables) disposable.dispose()
    }
    group.state.diagnosticsDisposable?.dispose()
    if (!session) return
    session.executeCommandsDisposable.dispose()
    session.applyEditDisposable.dispose()
    session.semanticTokensRefreshDisposable.dispose()
    session.codeLensRefreshDisposable.dispose()
    session.client.dispose()
    recordsBySessionId.delete(session.sessionId)
    await Promise.all(Array.from(group.roots).map((root) => stopLspSession(session.sessionId, root).catch(() => undefined)))
}

/**
 * Removes every `sessionsByKey` entry aliasing `group` (one per root joined onto it) and disposes
 * it — the terminal step of both the grace-period timer firing (`releaseLspSession`) and a forced,
 * bypass-the-timer teardown (`flushLspSessionDisposal`/`flushLspSessionsForProject`), so every path
 * ends up in the exact same disposed state. `record.group !== group` (guarding both the key removal
 * and the dispose) makes a *stale* call — one armed against `record`'s group before a later join
 * reassigned `record.group` elsewhere — a safe no-op: whatever group `record` now actually belongs
 * to has its own independent refcount/timer this call knows nothing about and must not touch.
 */
const finalizeSessionDisposal = (record: SessionRecord, group: SessionGroup) => {
    if (record.group !== group) return
    for (const root of group.roots) {
        const key = toSessionKey(group.projectId, group.serverId, root)
        if (sessionsByKey.get(key)?.group === group) sessionsByKey.delete(key)
    }
    void disposeSession(record, group)
}

export const acquireLspSession = (
    projectId: ProjectId,
    serverId: LspServerId,
    root: string,
    initializationOptions?: LspInitializationOptionsValue | null,
) => {
    const key = toSessionKey(projectId, serverId, root)
    const existing = sessionsByKey.get(key)
    if (existing) {
        if (existing.group.disposeTimer !== null) {
            clearTimeout(existing.group.disposeTimer)
            existing.group.disposeTimer = null
        }
        existing.group.refCount += 1
        return { key, record: existing }
    }

    const sibling = findAnyRecordForServer(projectId, serverId)
    const group: SessionGroup = {
        projectId,
        serverId,
        initializationOptions,
        refCount: 1,
        disposeTimer: null,
        roots: new Set(),
        state: createConnectionState(),
        lastObservedGeneration: 0,
    }

    let resolveReady: (value: ResolvedSession) => void = () => {}
    let rejectReady: (reason?: unknown) => void = () => {}
    const ready = new Promise<ResolvedSession>((resolve, reject) => {
        resolveReady = resolve
        rejectReady = reject
    })
    const record: SessionRecord = { group, ready }
    createSession(record, sibling, projectId, serverId, root, initializationOptions).then(resolveReady, rejectReady)

    void ready.catch(() => {
        if (sessionsByKey.get(key) === record) sessionsByKey.delete(key)
    })
    sessionsByKey.set(key, record)
    notifyWaiters(toWaiterKey(projectId, serverId))
    return { key, record }
}

export const peekLspSession = (projectId: ProjectId, serverId: LspServerId) => findAnyRecordForServer(projectId, serverId)

/**
 * Every currently-acquired session for `projectId`, across every server/language — used by the
 * command palette's `⌘T` Workspace Symbol search (`command-palette.tsx`), which queries all of a
 * project's active sessions in parallel rather than one language's sessions like `waitForLspSession`
 * callers (outline, `@` mode) do. Deduplicated by `SessionRecord` reference: a `sharesSessions`
 * connection joined by several roots (R7#7) appears under several `sessionsByKey` keys, but it is
 * one live connection worth querying once, not once per joined root.
 */
export const listSessionRecordsForProject = (projectId: ProjectId): SessionRecord[] => {
    const prefix = `${projectId}::`
    const seen = new Set<SessionRecord>()
    for (const [key, record] of sessionsByKey) {
        if (key.startsWith(prefix)) seen.add(record)
    }
    return Array.from(seen)
}

export const waitForLspSession = (projectId: ProjectId, serverId: LspServerId) => {
    const waiterKey = toWaiterKey(projectId, serverId)
    const existing = findAnyRecordForServer(projectId, serverId)
    if (existing) return { promise: Promise.resolve<SessionRecord | null>(existing), cancel: () => {} }

    let waiter: () => void = () => {}
    const promise = new Promise<SessionRecord | null>((resolve) => {
        waiter = () => resolve(findAnyRecordForServer(projectId, serverId))
        const waiters = waitersByKey.get(waiterKey) ?? new Set()
        waiters.add(waiter)
        waitersByKey.set(waiterKey, waiters)
    })
    const cancel = () => waitersByKey.get(waiterKey)?.delete(waiter)
    return { promise, cancel }
}

/**
 * `graceMs` defaults to {@link LSP_SESSION_DISPOSE_GRACE_MS} and exists as a parameter purely so
 * tests can shrink the wait (this codebase's established pattern for timer-driven modules, see
 * `enterKeymapChordPending`/`armKeymapMonacoDeferral` in `keymap-chord-store.ts`) — production call
 * sites never pass it. `key` is accepted for API-shape symmetry with `acquireLspSession`'s return
 * value (and matches every other function here taking the `{ key, record }` pair) but unused —
 * disposal is keyed off `record.group` (shared across every root joined onto the same connection,
 * R7#7), not the single root-specific key this particular acquisition happened to be made under.
 */
export const releaseLspSession = (_key: string, record: SessionRecord, graceMs: number = LSP_SESSION_DISPOSE_GRACE_MS) => {
    const group = record.group
    group.refCount -= 1
    if (group.refCount > 0) return
    group.disposeTimer = setTimeout(() => {
        group.disposeTimer = null
        finalizeSessionDisposal(record, group)
    }, graceMs)
}

/**
 * Cancels a session's pending grace-period timer (if any) and disposes it immediately — for
 * teardown paths where waiting out {@link LSP_SESSION_DISPOSE_GRACE_MS} would be wrong: definitive
 * project-close / full-app-exit (`kill_all`, hot-exit flush) / explicit-stop cleanup, as opposed to
 * the everyday file-switch the grace period exists to absorb. A no-op if `record`'s group never
 * entered grace (still has an active `refCount`) or was already disposed. `key` is unused for the
 * same reason `releaseLspSession`'s is — see its doc comment.
 */
export const flushLspSessionDisposal = (_key: string, record: SessionRecord) => {
    const group = record.group
    if (group.disposeTimer === null) return
    clearTimeout(group.disposeTimer)
    group.disposeTimer = null
    finalizeSessionDisposal(record, group)
}

/**
 * Force-disposes every session key-scoped to `projectId`, regardless of `refCount` or grace-period
 * state — `project_close` (Rust) removes the project's layout/watchers but never touches
 * `LspStore`, so without this a closed project's language servers would otherwise linger for the
 * full {@link LSP_SESSION_DISPOSE_GRACE_MS} after their last editor pane unmounts, `lsp_stop`-ing
 * against a project the user already explicitly closed. This deliberately does **not** delegate to
 * {@link flushLspSessionDisposal} (which only acts on a session already in its grace period,
 * `disposeTimer !== null`): `events.projectClosed` is delivered synchronously from a Tauri IPC
 * callback (`ipc-sync-provider.tsx`), not from a React commit, so it always runs *before* the
 * closing project's own panes have unmounted and released their sessions — at that instant every
 * one of them is still `refCount >= 1` with no `disposeTimer` set, which made a grace-gated flush a
 * structural no-op at its only real call site. Forcing disposal here is safe specifically because
 * the whole project is going away: every pane that could still reacquire this session is about to
 * unmount too, so there is no later reacquisition worth waiting for. A pane's own cleanup
 * (`releaseLspSession`) still runs afterward and is harmless against an already-disposed record —
 * `finalizeSessionDisposal`/`disposeSession` are idempotent (monaco disposables and `stopLspSession`
 * tolerate a second call). Deduplicated by group (R7#7) so a connection joined by several of this
 * project's roots is finalized once, not once per root.
 */
export const flushLspSessionsForProject = (projectId: ProjectId) => {
    const prefix = `${projectId}::`
    const finalized = new Set<SessionGroup>()
    for (const [key, record] of sessionsByKey) {
        if (!key.startsWith(prefix) || finalized.has(record.group)) continue
        finalized.add(record.group)
        finalizeSessionDisposal(record, record.group)
    }
}

/**
 * Flushes every session currently in its grace period, regardless of project — called from the
 * hot-exit flush handshake (`HotExitFlushProvider`) so a session mid-grace when the app starts
 * quitting sends its graceful `shutdown`/`exit` JSON-RPC sequence instead of only ever being
 * reaped by `LspStore::kill_all`'s unconditional process kill on `RunEvent::Exit`. Deduplicated by
 * group (R7#7) for the same reason `flushLspSessionsForProject` is.
 */
export const flushAllLspSessionDisposals = () => {
    const flushed = new Set<SessionGroup>()
    for (const [key, record] of sessionsByKey) {
        if (flushed.has(record.group)) continue
        flushed.add(record.group)
        flushLspSessionDisposal(key, record)
    }
}

registerLspSessionAllFlush(flushAllLspSessionDisposals)
registerLspSessionProjectFlush(flushLspSessionsForProject)

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
    const state = record.group.state
    if (!state.diagnosticsDisposable) state.diagnosticsDisposable = registerDiagnostics(monaco, client, serverId)
    if (state.languageDisposables.has(languageId)) return

    const disposables = [
        ...LANGUAGE_ADAPTER_REGISTRARS.map((register) => register(monaco, client, languageId)),
        registerCodeAction(monaco, client, serverId, languageId),
        registerCodeLens(monaco, client, languageId, isCodeLensEnabled),
        registerSemanticTokens(monaco, client, languageId, isSemanticHighlightingEnabled),
    ]
    state.languageDisposables.set(languageId, disposables)
    for (const listener of languageAdapterListeners) listener()
}

export const acquireDocument = (record: SessionRecord, client: LspClient, uri: string, languageId: string, text: string) => {
    const openDocuments = record.group.state.openDocuments
    const current = openDocuments.get(uri) ?? 0
    openDocuments.set(uri, current + 1)
    if (current === 0) client.didOpen({ uri, languageId, version: 0, text })
}

export const releaseDocument = (record: SessionRecord, client: LspClient, uri: string) => {
    const openDocuments = record.group.state.openDocuments
    const current = openDocuments.get(uri)
    if (!current) return
    if (current <= 1) {
        openDocuments.delete(uri)
        client.didClose(uri)
        return
    }
    openDocuments.set(uri, current - 1)
}

/**
 * R7#1 — re-handshakes a session whose backend process was silently replaced by
 * `handle_process_exit`'s automatic crash-restart (the `LspSessionStatusChanged.generation`
 * increase {@link handleLspSessionStatusChanged} detected). The `session_id` is unchanged (Rust
 * reuses it for the respawned process), so this reuses the *same* `LspClient`/monaco provider
 * registrations rather than tearing anything down:
 *  1. `rejectPendingRequests` — any request still awaiting a response from the now-dead pre-crash
 *     process can never get one; leaving it pending would hang whatever awaited it forever.
 *  2. Re-run `initialize` over the same connection (LSP 3.17 requires exactly one `initialize` per
 *     connection; the respawned process has never seen one).
 *  3. Rebuild `executeCommandsDisposable` — the command list `registerSessionExecuteCommands`
 *     registered was a one-time snapshot of the *pre-crash* process's `executeCommandProvider`,
 *     which the respawned process may declare differently. Every other adapter reads
 *     `client.supports(...)`/`getCapabilities()` fresh per call, so they need no rebuild.
 *  4. Re-send `didOpen` for every document this connection has open — the respawned process has no
 *     memory of any previously-open document, and without this it silently has none.
 *  5. `lsp_confirm_reinitialize` — the only thing that flips `status` back to `Running` (guarded on
 *     the Rust side against a stale confirmation racing a second crash — see the `generation` field
 *     doc on `domain::lsp::commands::SessionEntry`).
 * Any failure along the way leaves `status` as `Crashed` (never calls step 5) rather than throwing
 * past this function — a later generation bump (a second auto-restart) or a manual `lsp_restart`
 * gets another attempt.
 */
const reinitializeSession = async (record: SessionRecord, group: SessionGroup, generation: number) => {
    const session = await record.ready.catch(() => null)
    if (!session) return
    try {
        session.client.rejectPendingRequests(new Error('lsp session reinitializing after crash'))
        await session.client.initialize(buildInitializeParams(group.roots, group.initializationOptions))
        session.executeCommandsDisposable.dispose()
        session.executeCommandsDisposable = registerSessionExecuteCommands(
            monaco,
            session.client,
            session.client.getCapabilities()?.executeCommandProvider?.commands,
        )
        for (const uri of group.state.openDocuments.keys()) {
            const model = monaco.editor.getModel(monaco.Uri.parse(uri))
            if (!model) continue
            session.client.didOpen({ uri, languageId: model.getLanguageId(), version: 0, text: model.getValue() })
        }
        await confirmLspReinitialize(session.sessionId, generation)
    } catch {
        return
    }
}

/**
 * Exported purely for tests to drive directly instead of going through the real Tauri event system
 * (see the module-level `events.lspSessionStatusChanged.listen` call below). Only a strictly
 * *increasing* `generation` triggers anything — a duplicate or out-of-order delivery of an
 * already-handled generation is a no-op, matching the guarantee `confirm_reinitialize` (Rust) makes
 * for the confirmation half of this same handshake. Only `status: 'crashed'` triggers the
 * reinitialize flow: `starting`/`running`/`stopped` carry a generation for completeness (the field
 * doc on `LspSessionStatusChanged.generation` — Rust bumps it only on the crash-restart path, so in
 * practice those other statuses never carry an increase at all) but need no client-side reaction.
 */
export const handleLspSessionStatusChanged = (payload: LspSessionStatusChanged) => {
    const record = recordsBySessionId.get(payload.sessionId)
    if (!record) return
    const group = record.group
    if (payload.generation <= group.lastObservedGeneration) return
    group.lastObservedGeneration = payload.generation
    if (payload.status !== 'crashed') return
    void reinitializeSession(record, group, payload.generation)
}

void events.lspSessionStatusChanged.listen(({ payload }) => handleLspSessionStatusChanged(payload)).catch(() => undefined)
