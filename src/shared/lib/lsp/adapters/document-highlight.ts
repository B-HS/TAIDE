import type { CancellationToken } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { NOOP_DISPOSABLE } from '@shared/lib/lsp/noop-disposable'
import type { DocumentHighlight, DocumentHighlightKind } from '@shared/lib/lsp/protocol'
import { DOCUMENT_HIGHLIGHT_KIND, isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const MONACO_HIGHLIGHT_KIND_BY_LSP_KIND: Record<DocumentHighlightKind, number> = {
    [DOCUMENT_HIGHLIGHT_KIND.TEXT]: 0,
    [DOCUMENT_HIGHLIGHT_KIND.READ]: 1,
    [DOCUMENT_HIGHLIGHT_KIND.WRITE]: 2,
}

export const toMonacoHighlightKind = (kind: DocumentHighlightKind | undefined) =>
    MONACO_HIGHLIGHT_KIND_BY_LSP_KIND[kind ?? DOCUMENT_HIGHLIGHT_KIND.TEXT]

export const toMonacoDocumentHighlight = (highlight: DocumentHighlight) => ({
    range: lspRangeToMonaco(highlight.range),
    kind: toMonacoHighlightKind(highlight.kind),
})

export const registerDocumentHighlight = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.documentHighlightProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerDocumentHighlightProvider(languageId, {
        provideDocumentHighlights: async (model, position, token: CancellationToken) => {
            const result = await client.request<DocumentHighlight[] | null>('textDocument/documentHighlight', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
            })
            if (token.isCancellationRequested || !result) return []
            return result.map(toMonacoDocumentHighlight)
        },
    })
}
