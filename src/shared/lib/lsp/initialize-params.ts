import type { LspInitializationOptionsValue } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { SEMANTIC_TOKEN_MODIFIERS, SEMANTIC_TOKEN_TYPES, SYMBOL_KIND_VALUE_SET } from '@shared/lib/lsp/protocol'
import { RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID, RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID } from '@shared/lib/lsp/command-relay'

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

/**
 * `roots` is every workspace root this connection currently serves — for a `sharesSessions` server
 * joined by more than one root (R7#7), `workspaceFolders` advertises all of them up front (matching
 * what `lsp_spawn`'s own `workspace/didChangeWorkspaceFolders` notification keeps in sync for roots
 * added *after* this initial handshake). The first root (insertion order) is used for the legacy
 * singular `rootUri`/`rootPath` fields LSP 3.17 deprecated in favor of `workspaceFolders` but many
 * servers still read.
 */
export const buildInitializeParams = (roots: ReadonlySet<string>, initializationOptions?: LspInitializationOptionsValue | null) => {
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
                 * is likewise registered *per session* on `client` in `createSession`
                 * (`@entities/lsp/lsp-session-registry.ts`) (F7#4; it used to be a single
                 * process-wide handler in `server-request-handler-registry.ts`, which fired every
                 * open session's listeners on a refresh push from any one server) — the semantic
                 * tokens refresh handler must be registered *per session* via
                 * `client.registerRequestHandler` (the `workspace/applyEdit` pattern already used in
                 * `createSession`, `@entities/lsp/lsp-session-registry.ts`), so a refresh request
                 * from one server only invalidates that server's own semantic tokens adapter instead
                 * of every open session's. That handler is wired in the same commit that registers
                 * the semantic tokens adapter itself (Wave F Phase D) — declaring this capability
                 * ahead of that handler would make the server's eventual refresh request bounce off
                 * as an unhandled `-32601 MethodNotFound`, so the two must land together.
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
