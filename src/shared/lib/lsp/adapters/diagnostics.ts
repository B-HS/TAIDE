import type { LspServerId } from '@shared/api/bindings'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Diagnostic } from '@shared/lib/lsp/protocol'
import { DIAGNOSTIC_SEVERITY } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco } from '@shared/lib/lsp/position'

let nextDiagnosticsOwnerSequence = 0
const ownerByClient = new WeakMap<LspClient, string>()

/**
 * Stable per-*session* (not per-server) monaco marker owner / `rawDiagnosticsByOwnerUri` store key
 * — two independent sessions of the *same* server id (e.g. two rust-analyzer processes for two
 * disjoint Cargo workspaces under one project, R7#7) must never share an owner tag, or one
 * session's diagnostics teardown/republish would stomp the other's `setModelMarkers` state (F7#5).
 * Keyed by `client` identity (a `WeakMap`, so an owner is reclaimed automatically once nothing else
 * references its client), which is stable for the lifetime of one underlying LSP connection —
 * including every root joined onto a `sharesSessions` server's single connection — rather than
 * `serverId` alone. Idempotent per client: the first caller (always `registerDiagnostics`, called
 * once per session via `ensureLanguageRegistered`'s `!state.diagnosticsDisposable` guard) mints the
 * owner; later callers (`registerCodeAction`, `getStoredDiagnostics`) just look it up.
 */
export const diagnosticsOwnerForClient = (serverId: LspServerId, client: LspClient) => {
    const existing = ownerByClient.get(client)
    if (existing) return existing
    nextDiagnosticsOwnerSequence += 1
    const owner = `lsp-${serverId}-${nextDiagnosticsOwnerSequence}`
    ownerByClient.set(client, owner)
    return owner
}

const toMonacoSeverity = (monaco: Monaco, severity: Diagnostic['severity']) => {
    if (severity === DIAGNOSTIC_SEVERITY.WARNING) return monaco.MarkerSeverity.Warning
    if (severity === DIAGNOSTIC_SEVERITY.INFORMATION) return monaco.MarkerSeverity.Info
    if (severity === DIAGNOSTIC_SEVERITY.HINT) return monaco.MarkerSeverity.Hint
    return monaco.MarkerSeverity.Error
}

const toMonacoMarker = (monaco: Monaco, diagnostic: Diagnostic) => ({
    ...lspRangeToMonaco(diagnostic.range),
    severity: toMonacoSeverity(monaco, diagnostic.severity),
    message: diagnostic.message,
    source: diagnostic.source,
    code: diagnostic.code === undefined ? undefined : String(diagnostic.code),
})

/**
 * Raw (pre-marker-conversion) diagnostics keyed by `${owner}::${uri}`, preserving `code`/`data`/
 * `source` that the lossy monaco marker conversion above drops. Code actions need these originals
 * back — gopls bundles lazy quickfixes in `diagnostic.data` and tsls keys its quickfixes off
 * `diagnostic.code`, both lost if `context.diagnostics` were rebuilt from monaco markers instead.
 */
const rawDiagnosticsByOwnerUri = new Map<string, Diagnostic[]>()

const toStoreKey = (owner: string, uri: string) => `${owner}::${uri}`

/**
 * Returns the last `publishDiagnostics` batch this server sent for `uri`, in original LSP shape.
 * `client` (the session whose diagnostics you want) is required and scopes the lookup to exactly
 * that session's own store (F7#5) — two independent sessions of the same `serverId` (R7#7) must
 * never leak each other's diagnostics into a code-action request through this lookup.
 */
export const getStoredDiagnostics = (serverId: LspServerId, uri: string, client: LspClient): Diagnostic[] =>
    rawDiagnosticsByOwnerUri.get(toStoreKey(diagnosticsOwnerForClient(serverId, client), uri)) ?? []

export const registerDiagnostics = (monaco: Monaco, client: LspClient, serverId: LspServerId) => {
    const owner = diagnosticsOwnerForClient(serverId, client)
    /**
     * uris this specific `registerDiagnostics` call (one LSP session's diagnostics stream) has
     * actually written, so `dispose` below can clean up only its own entries. `owner` is already
     * session-scoped (`diagnosticsOwnerForClient`, F7#5), but this per-call set still matters: a
     * scan-and-delete-every-`${owner}::`-key dispose would be redundant work, and keeping the exact
     * set this session wrote makes the cleanup O(this session's own uris) instead of O(everything
     * ever stored for this owner).
     */
    const ownedUris = new Set<string>()

    const modelDisposalDisposable = monaco.editor.onWillDisposeModel((model) => {
        const uri = model.uri.toString()
        rawDiagnosticsByOwnerUri.delete(toStoreKey(owner, uri))
        ownedUris.delete(uri)
    })

    const notificationDisposable = client.onDiagnostics((params) => {
        /**
         * Normalized to monaco's own uri representation before storage — `getStoredDiagnostics`
         * callers always look up with `model.uri.toString()` (already monaco-normalized), while
         * `params.uri` is the server's own wire string. The two differ for paths containing
         * characters monaco percent-encodes but the server's own uri library does not (e.g. `(`,
         * `)`, `,`), which would otherwise make every lookup for such a path silently miss.
         */
        const normalizedUri = monaco.Uri.parse(params.uri).toString()
        rawDiagnosticsByOwnerUri.set(toStoreKey(owner, normalizedUri), params.diagnostics)
        ownedUris.add(normalizedUri)
        const model = monaco.editor.getModel(monaco.Uri.parse(params.uri))
        if (!model) return
        monaco.editor.setModelMarkers(
            model,
            owner,
            params.diagnostics.map((diagnostic) => toMonacoMarker(monaco, diagnostic)),
        )
    })

    return {
        /**
         * Besides dropping this session's own raw-diagnostics entries, also clears every marker it
         * ever wrote (`setModelMarkers(model, owner, [])`) on the models still alive for `ownedUris`
         * — `owner` is minted fresh per session (`diagnosticsOwnerForClient`), so unlike the old
         * shared-per-server owner (which the *next* session's first publish silently overwrote),
         * nothing else will ever republish under this exact owner again. Without this, a session
         * disposed by the everyday grace-period file-switch (`LSP_SESSION_DISPOSE_GRACE_MS`) leaves
         * its markers on the model forever, and every subsequent session for the same file/server
         * adds its own on top — diagnostics counts double every cycle. A model already torn down by
         * the time `dispose` runs was already removed from `ownedUris` by `modelDisposalDisposable`
         * above, so `getModel` returning nothing here is expected, not an error.
         */
        dispose: () => {
            notificationDisposable.dispose()
            modelDisposalDisposable.dispose()
            for (const uri of ownedUris) {
                rawDiagnosticsByOwnerUri.delete(toStoreKey(owner, uri))
                const model = monaco.editor.getModel(monaco.Uri.parse(uri))
                if (model) monaco.editor.setModelMarkers(model, owner, [])
            }
        },
    }
}
