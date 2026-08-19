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
 * actual increase, never a duplicate/out-of-order delivery of an already-handled one. `isReinitializing`
 * is `true` for the duration of {@link reinitializeSession}'s retry loop — `acquireDocument`/
 * `releaseDocument` read it to avoid racing that loop's own `didOpen` replay with a document
 * opened/closed by the user mid-replay (see their own doc comments for exactly what each does and
 * does not still send while this is `true`).
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
    isReinitializing: boolean
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
 *
 * Correct for `acquireLspSession`'s own sibling-join lookup (any existing root is a valid join
 * candidate there — the backend, not this pick, decides whether the connection is actually shared,
 * see `createSession`'s doc). **Not** correct in general for a caller that has a specific document
 * open and wants *that document's* session: since session keys include `root` (R7#7), a project with
 * more than one root open under the same `(projectId, serverId)` — routine for a `shares_sessions:
 * false` server like rustAnalyzer with two disjoint Cargo workspaces — can have several independent
 * records here, and this always returns the oldest one regardless of which root actually has the
 * caller's file open. {@link peekLspSession}/{@link waitForLspSession} inherit that same limitation;
 * prefer {@link peekLspSessionForRoot}/{@link waitForLspSessionForRoot} whenever a root is available.
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

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))

/**
 * Races `promise` against a `ms`-long timer, rejecting with `message` if the timer wins — the only
 * way to turn an LSP round-trip that never settles (`client.ts`'s `request` has no built-in timeout
 * of its own) into a rejection callers here can actually catch and act on. Always clears its own
 * timer on either outcome so a fast-settling `promise` doesn't leave a dangling `setTimeout` behind.
 */
const withTimeout = async <T>(promise: Promise<T>, ms: number, message: string): Promise<T> => {
    let timeoutId: ReturnType<typeof setTimeout> | undefined
    const timeout = new Promise<never>((_, reject) => {
        timeoutId = setTimeout(() => reject(new Error(message)), ms)
    })
    try {
        return await Promise.race([promise, timeout])
    } finally {
        clearTimeout(timeoutId)
    }
}

/**
 * Bounds `createSession`'s `await sibling.ready` (see that call site's doc) so a sibling root whose
 * own first `initialize` handshake genuinely never answers cannot indefinitely block every other root
 * of the same `(projectId, serverId)` from ever attempting its own spawn — generous (well past any
 * healthy server's typical handshake, rust-analyzer's initial workspace index included) since timing
 * out here is not free: it forfeits join detection for *this* attempt, and the two roots' backend
 * connections stay independent for `shares_sessions: true` servers that would otherwise have shared
 * one (`should_reuse_session`, Rust, tolerates this — the docs on `record.group.roots` field cover
 * why a missed join is a session-count regression, not a correctness one).
 */
const LSP_SIBLING_READY_TIMEOUT_MS = 20_000

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
 *
 * The `sibling.ready` wait is bounded by {@link LSP_SIBLING_READY_TIMEOUT_MS} (see its own doc for
 * why unboundedly serializing every root behind `sibling`'s handshake is unsafe for a
 * `shares_sessions: false` server with more than one root, and why timing out here — rather than
 * waiting forever — is the safe direction to fail in).
 */
const createSession = async (
    record: SessionRecord,
    sibling: SessionRecord | null,
    projectId: ProjectId,
    serverId: LspServerId,
    root: string,
    initializationOptions?: LspInitializationOptionsValue | null,
    siblingReadyTimeoutMs: number = LSP_SIBLING_READY_TIMEOUT_MS,
): Promise<ResolvedSession> => {
    if (sibling) await withTimeout(sibling.ready, siblingReadyTimeoutMs, 'sibling lsp session ready wait timed out').catch(() => undefined)

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
        /**
         * `record.group.refCount` (this handle's own, still-private group, before the line above
         * repoints it) can itself already be 0 — this handle's own `releaseLspSession` may have run
         * while `spawnLspSession` above was still in flight, well before this join was ever
         * discovered. In that case the combined `joined.group.refCount` above lands back at 0 too,
         * but the timer clear a few lines up already cancelled whatever grace timer `joined.group`
         * had running *without rearming one* — without this, a session joined while already fully
         * idle on both sides would never get GC'd by the ordinary grace path again (only an explicit
         * project-close/app-exit flush would ever reclaim it).
         */
        if (joined.group.refCount <= 0) {
            const group = joined.group
            group.disposeTimer = setTimeout(() => {
                group.disposeTimer = null
                finalizeSessionDisposal(record, group)
            }, LSP_SESSION_DISPOSE_GRACE_MS)
        }
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
 *
 * Scans `sessionsByKey` directly (matching every entry whose own `.group === group`) rather than
 * recomputing each root's key from `group.roots` — `roots` is only populated once `createSession`'s
 * `spawnLspSession` call resolves (see its own field doc), so a group disposed *while that spawn is
 * still in flight* (a project closing/pane unmounting mid-spawn) had an empty `roots` here despite
 * `acquireLspSession` having already inserted its `sessionsByKey` entry. Recomputing from `roots`
 * silently left that entry behind forever in that window; scanning the map's actual contents finds
 * it regardless of whether `roots` has caught up yet.
 */
const finalizeSessionDisposal = (record: SessionRecord, group: SessionGroup) => {
    if (record.group !== group) return
    for (const [key, entry] of sessionsByKey) {
        if (entry.group === group) sessionsByKey.delete(key)
    }
    void disposeSession(record, group)
}

/**
 * `siblingReadyTimeoutMs` defaults to {@link LSP_SIBLING_READY_TIMEOUT_MS} and exists as a parameter
 * purely so tests can shrink it (this file's established pattern — see {@link releaseLspSession}'s
 * `graceMs` doc); production call sites never override it.
 */
export const acquireLspSession = (
    projectId: ProjectId,
    serverId: LspServerId,
    root: string,
    initializationOptions?: LspInitializationOptionsValue | null,
    siblingReadyTimeoutMs: number = LSP_SIBLING_READY_TIMEOUT_MS,
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
        isReinitializing: false,
    }

    let resolveReady: (value: ResolvedSession) => void = () => {}
    let rejectReady: (reason?: unknown) => void = () => {}
    const ready = new Promise<ResolvedSession>((resolve, reject) => {
        resolveReady = resolve
        rejectReady = reject
    })
    const record: SessionRecord = { group, ready }
    createSession(record, sibling, projectId, serverId, root, initializationOptions, siblingReadyTimeoutMs).then(resolveReady, rejectReady)

    void ready.catch(() => {
        if (sessionsByKey.get(key) === record) sessionsByKey.delete(key)
    })
    sessionsByKey.set(key, record)
    notifyWaiters(toWaiterKey(projectId, serverId))
    return { key, record }
}

/**
 * Root-agnostic: returns *some* session for `(projectId, serverId)` — the oldest-inserted one
 * ({@link findAnyRecordForServer}), not necessarily the one that has the caller's file open. Correct
 * whenever `(projectId, serverId)` can only ever have one root's session at a time, and for callers
 * that are deliberately root-agnostic (`listSessionRecordsForProject`'s workspace-wide sweep). Wrong
 * whenever a project has more than one root open for a `shares_sessions: false` server (R7#7) and the
 * caller actually needs the session serving a *specific* file — use {@link peekLspSessionForRoot}
 * with that file's resolved root instead.
 */
export const peekLspSession = (projectId: ProjectId, serverId: LspServerId) => findAnyRecordForServer(projectId, serverId)

/**
 * Root-exact counterpart to {@link peekLspSession} — returns the session acquired for this precise
 * `(projectId, serverId, root)` triple, or `null` if none has been acquired. Correct in the
 * multi-root case {@link peekLspSession} is not: a `sharesSessions` server's join (R7#7) reassigns
 * the joining root's own `SessionRecord.group` in place rather than replacing the `sessionsByKey`
 * entry, so the record found here for a joined root already resolves to the *same* shared
 * `ResolvedSession` its sibling roots do — this never returns a "wrong session" the way
 * {@link peekLspSession} can when several independent (non-shared) roots coexist.
 */
export const peekLspSessionForRoot = (projectId: ProjectId, serverId: LspServerId, root: string): SessionRecord | null =>
    sessionsByKey.get(toSessionKey(projectId, serverId, root)) ?? null

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

/**
 * Root-agnostic session waiter — same "some session for `(projectId, serverId)`, not necessarily the
 * caller's root" caveat as {@link peekLspSession} (see its doc) applies once resolved, since a waiter
 * registered before any session exists yet has no root to disambiguate by either. Prefer
 * {@link waitForLspSessionForRoot} whenever the caller can resolve a root.
 */
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
 * Root-exact counterpart to {@link waitForLspSession}, mirroring {@link peekLspSessionForRoot}'s
 * exact-key lookup instead of {@link findAnyRecordForServer}'s oldest-match. Shares the underlying
 * `(projectId, serverId)` waiter queue (session keys carry `root`, but nothing currently indexes
 * waiters by root too) — a waiter registered here still wakes on *any* new session for this server,
 * then re-checks the exact `(projectId, serverId, root)` key and resolves `null` if that specific
 * root's session is not the one that just got created. A caller that must keep waiting past that
 * still has the returned `cancel` to drop this attempt and register a fresh one.
 */
export const waitForLspSessionForRoot = (projectId: ProjectId, serverId: LspServerId, root: string) => {
    const key = toSessionKey(projectId, serverId, root)
    const waiterKey = toWaiterKey(projectId, serverId)
    const existing = sessionsByKey.get(key)
    if (existing) return { promise: Promise.resolve<SessionRecord | null>(existing), cancel: () => {} }

    let waiter: () => void = () => {}
    const promise = new Promise<SessionRecord | null>((resolve) => {
        waiter = () => resolve(sessionsByKey.get(key) ?? null)
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

/**
 * While `record.group.isReinitializing` is `true` ({@link reinitializeSession} mid-retry-loop), a
 * genuinely new document (`current === 0`) still gets counted here but does *not* get its own
 * `didOpen` sent — `openDocuments` is exactly what that loop's own live-map iteration replays, so a
 * document added mid-replay is naturally picked up by it (or, if added just after this attempt's
 * iterator already passed it, by the next attempt's fresh iteration) without this call also sending
 * its own `didOpen` and racing a double-send for the same document (F7#1 follow-up).
 */
export const acquireDocument = (record: SessionRecord, client: LspClient, uri: string, languageId: string, text: string) => {
    const openDocuments = record.group.state.openDocuments
    const current = openDocuments.get(uri) ?? 0
    openDocuments.set(uri, current + 1)
    if (current === 0 && !record.group.isReinitializing) client.didOpen({ uri, languageId, version: 0, text })
}

/**
 * Mirrors {@link acquireDocument}'s `isReinitializing` gate: closing the last reference to `uri`
 * while a reinitialize retry loop is running does not send `didClose` — the respawned process may
 * not have received this document's replayed `didOpen` yet (queued behind an in-progress or still
 * up-coming attempt), and a `didClose` for a document a server was never told about is itself a
 * protocol violation. Deleting `uri` from `openDocuments` here still removes it from what the
 * (live-iterating) replay loop will send, so a document closed before its replay turn simply never
 * gets opened on the new process either — no leaked reference in the common case. The one residual
 * gap this does not close: a document whose `didOpen` the replay loop *already* sent earlier in the
 * same attempt, then closed by the user before that attempt's loop as a whole finishes, leaves the
 * new process believing it's still open (no compensating `didClose`) until the next real edit to
 * that document's session state — accepted as a narrow, low-impact residual rather than adding a
 * second deferred-close queue for a crash-recovery path already carrying real complexity.
 */
export const releaseDocument = (record: SessionRecord, client: LspClient, uri: string) => {
    const openDocuments = record.group.state.openDocuments
    const current = openDocuments.get(uri)
    if (!current) return
    if (current <= 1) {
        openDocuments.delete(uri)
        if (!record.group.isReinitializing) client.didClose(uri)
        return
    }
    openDocuments.set(uri, current - 1)
}

/**
 * How long a single re-handshake attempt (the `initialize` round-trip {@link reinitializeSession}
 * sends over the respawned process's connection) may take before it's abandoned as failed.
 * `client.ts`'s `request` has no timeout of its own — a respawned process that never answers
 * `initialize` would otherwise leave the `await` below pending forever, so this bounds specifically
 * the reinitialize handshake without changing every other LSP request's own (timeout-less) contract.
 */
const LSP_REINITIALIZE_TIMEOUT_MS = 15_000

/**
 * How many re-handshake attempts {@link reinitializeSession} makes for one `generation` before
 * giving up on it — without this, a single transient failure (the respawned process not yet ready to
 * accept connections, a dropped first request) left the session `Crashed` forever with no further
 * trigger: nothing but a *second* crash-restart (an unrelated backend event) or a manual
 * `lsp_restart` (which nothing in the UI currently calls) would ever attempt another `initialize`.
 */
const LSP_REINITIALIZE_MAX_ATTEMPTS = 3

/** Backoff between {@link LSP_REINITIALIZE_MAX_ATTEMPTS} retries — long enough to let a just-spawned process finish booting before the next attempt. */
const LSP_REINITIALIZE_RETRY_DELAY_MS = 2_000

/**
 * R7#1 — re-handshakes a session whose backend process was silently replaced by
 * `handle_process_exit`'s automatic crash-restart (the `LspSessionStatusChanged.generation`
 * increase {@link handleLspSessionStatusChanged} detected). The `session_id` is unchanged (Rust
 * reuses it for the respawned process), so this reuses the *same* `LspClient`/monaco provider
 * registrations rather than tearing anything down. Each of up to {@link LSP_REINITIALIZE_MAX_ATTEMPTS}
 * attempts:
 *  1. `rejectPendingRequests` — any request still awaiting a response from the now-dead pre-crash
 *     process (first attempt) or a previous attempt's own timed-out `initialize` (later attempts)
 *     can never get one; leaving it pending would hang whatever awaited it forever.
 *  2. Re-run `initialize` over the same connection (LSP 3.17 requires exactly one *answered*
 *     `initialize` per connection; the respawned process has never seen one), bounded by
 *     {@link withTimeout}/{@link LSP_REINITIALIZE_TIMEOUT_MS} so a process that never answers fails
 *     this attempt instead of hanging the whole flow.
 *  3. Rebuild `executeCommandsDisposable` — the command list `registerSessionExecuteCommands`
 *     registered was a one-time snapshot of the *pre-crash* process's `executeCommandProvider`,
 *     which the respawned process may declare differently. Every other adapter reads
 *     `client.supports(...)`/`getCapabilities()` fresh per call, so they need no rebuild.
 *  4. Re-send `didOpen` for every document this connection has open — the respawned process has no
 *     memory of any previously-open document, and without this it silently has none.
 *  5. `lsp_confirm_reinitialize` — the only thing that flips `status` back to `Running` (guarded on
 *     the Rust side against a stale confirmation racing a second crash — see the `generation` field
 *     doc on `domain::lsp::commands::SessionEntry`).
 * Before each attempt (after the first), re-checks `group.lastObservedGeneration` against the
 * `generation` this flow was started for — a newer crash while this loop was still retrying means a
 * fresher `reinitializeSession` call already owns (or has already won) the handshake for the process
 * currently running, and this stale loop must stop rather than race it with an out-of-date attempt.
 * Exhausting every attempt (or losing the staleness race) leaves `status` as `Crashed` rather than
 * throwing past this function — Rust's `last_error` has no failure-vs-still-retrying distinction to
 * flip to here (that requires a backend-side confirm/report command this frontend-only fix cannot
 * add), so a later generation bump (a second auto-restart) or a manual `lsp_restart` remains the only
 * further recovery path.
 *
 * `group.isReinitializing` is `true` for the entire loop below (`finally`-reset on every exit path,
 * including the staleness `return`) — see {@link acquireDocument}/{@link releaseDocument}'s own doc
 * comments for what that gate does and does not still send while a document opens/closes mid-replay.
 *
 * `timeoutMs`/`maxAttempts`/`retryDelayMs` default to {@link LSP_REINITIALIZE_TIMEOUT_MS}/
 * {@link LSP_REINITIALIZE_MAX_ATTEMPTS}/{@link LSP_REINITIALIZE_RETRY_DELAY_MS} and exist as
 * parameters purely so tests can shrink them (this file's established pattern — see
 * {@link releaseLspSession}'s `graceMs` doc); production call sites never override them.
 */
const reinitializeSession = async (
    record: SessionRecord,
    group: SessionGroup,
    generation: number,
    timeoutMs: number = LSP_REINITIALIZE_TIMEOUT_MS,
    maxAttempts: number = LSP_REINITIALIZE_MAX_ATTEMPTS,
    retryDelayMs: number = LSP_REINITIALIZE_RETRY_DELAY_MS,
) => {
    const session = await record.ready.catch(() => null)
    if (!session) return

    group.isReinitializing = true
    try {
        for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
            if (group.lastObservedGeneration > generation) return
            try {
                session.client.rejectPendingRequests(new Error('lsp session reinitializing after crash'))
                await withTimeout(
                    session.client.initialize(buildInitializeParams(group.roots, group.initializationOptions)),
                    timeoutMs,
                    'lsp reinitialize handshake timed out',
                )
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
                return
            } catch {
                if (attempt === maxAttempts) return
                await delay(retryDelayMs)
            }
        }
    } finally {
        group.isReinitializing = false
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
 * `reinitializeTiming` forwards to {@link reinitializeSession}'s own test-only timing overrides —
 * see its doc — and is likewise never passed by the real event listener below.
 */
export const handleLspSessionStatusChanged = (
    payload: LspSessionStatusChanged,
    reinitializeTiming?: { timeoutMs?: number; maxAttempts?: number; retryDelayMs?: number },
) => {
    const record = recordsBySessionId.get(payload.sessionId)
    if (!record) return
    const group = record.group
    if (payload.generation <= group.lastObservedGeneration) return
    group.lastObservedGeneration = payload.generation
    if (payload.status !== 'crashed') return
    void reinitializeSession(
        record,
        group,
        payload.generation,
        reinitializeTiming?.timeoutMs,
        reinitializeTiming?.maxAttempts,
        reinitializeTiming?.retryDelayMs,
    )
}

void events.lspSessionStatusChanged.listen(({ payload }) => handleLspSessionStatusChanged(payload)).catch(() => undefined)
