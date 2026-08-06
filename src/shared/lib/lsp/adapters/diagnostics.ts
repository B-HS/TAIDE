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

export const registerDiagnostics = (monaco: Monaco, client: LspClient, serverId: LspServerId) => {
    const owner = diagnosticsOwnerFor(serverId)
    return client.onDiagnostics((params) => {
        const model = monaco.editor.getModel(monaco.Uri.parse(params.uri))
        if (!model) return
        monaco.editor.setModelMarkers(
            model,
            owner,
            params.diagnostics.map((diagnostic) => toMonacoMarker(monaco, diagnostic)),
        )
    })
}
