import type { LspServerId } from '@shared/api/bindings'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Diagnostic } from '@shared/lib/lsp/protocol'
import { DIAGNOSTIC_SEVERITY } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco } from '@shared/lib/lsp/position'

export const diagnosticsOwnerFor = (serverId: LspServerId) => `lsp-${serverId}`

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

/** Returns the last `publishDiagnostics` batch this server sent for `uri`, in original LSP shape. */
export const getStoredDiagnostics = (serverId: LspServerId, uri: string) =>
    rawDiagnosticsByOwnerUri.get(toStoreKey(diagnosticsOwnerFor(serverId), uri)) ?? []

export const registerDiagnostics = (monaco: Monaco, client: LspClient, serverId: LspServerId) => {
    const owner = diagnosticsOwnerFor(serverId)
    /**
     * uris this specific `registerDiagnostics` call (one LSP session's diagnostics stream) has
     * actually written, so `dispose` below can clean up only its own entries. `owner` alone
     * (`lsp-${serverId}`) does not identify a session — two projects open at once can each run
     * their own rust-analyzer session, both reporting diagnostics under the same `owner`. Scanning
     * and deleting every `${owner}::` key on dispose (the previous approach) would wipe the other
     * project's still-live diagnostics the moment either session's tore down.
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
        dispose: () => {
            notificationDisposable.dispose()
            modelDisposalDisposable.dispose()
            for (const uri of ownedUris) rawDiagnosticsByOwnerUri.delete(toStoreKey(owner, uri))
        },
    }
}
