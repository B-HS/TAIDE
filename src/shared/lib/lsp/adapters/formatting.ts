import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { TextEdit } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

const toMonacoTextEdits = (edits: TextEdit[]) => edits.map((edit) => ({ range: lspRangeToMonaco(edit.range), text: edit.newText }))

export const registerFormatting = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.documentFormattingProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerDocumentFormattingEditProvider(languageId, {
        provideDocumentFormattingEdits: async (model, options) => {
            const result = await client.request<TextEdit[] | null>('textDocument/formatting', {
                textDocument: { uri: model.uri.toString() },
                options: { tabSize: options.tabSize, insertSpaces: options.insertSpaces },
            })
            return result ? toMonacoTextEdits(result) : []
        },
    })
}
