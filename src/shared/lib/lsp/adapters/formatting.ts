import type { CancellationToken } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { TextEdit } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp, monacoRangeToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

const toMonacoTextEdits = (edits: TextEdit[]) => edits.map((edit) => ({ range: lspRangeToMonaco(edit.range), text: edit.newText }))

export const registerFormatting = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.documentFormattingProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerDocumentFormattingEditProvider(languageId, {
        provideDocumentFormattingEdits: async (model, options, token: CancellationToken) => {
            const result = await client.request<TextEdit[] | null>('textDocument/formatting', {
                textDocument: { uri: model.uri.toString() },
                options: { tabSize: options.tabSize, insertSpaces: options.insertSpaces },
            })
            if (token.isCancellationRequested || !result) return []
            return toMonacoTextEdits(result)
        },
    })
}

/**
 * Registers the LSP range-formatting provider for `languageId`. Besides serving explicit
 * "Format Selection", this is the capability monaco's `formatOnPaste` hard-requires (no fallback
 * to whole-document formatting) — see {@link CodeEditorProps.formatOnPaste} in `code-editor.tsx`.
 */
export const registerRangeFormatting = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.documentRangeFormattingProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerDocumentRangeFormattingEditProvider(languageId, {
        provideDocumentRangeFormattingEdits: async (model, range, options, token: CancellationToken) => {
            const result = await client.request<TextEdit[] | null>('textDocument/rangeFormatting', {
                textDocument: { uri: model.uri.toString() },
                range: monacoRangeToLsp(range),
                options: { tabSize: options.tabSize, insertSpaces: options.insertSpaces },
            })
            if (token.isCancellationRequested || !result) return []
            return toMonacoTextEdits(result)
        },
    })
}

export const registerOnTypeFormatting = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.documentOnTypeFormattingProvider))) return NOOP_DISPOSABLE

    const capability = client.getCapabilities()?.documentOnTypeFormattingProvider
    if (!capability) return NOOP_DISPOSABLE

    return monaco.languages.registerOnTypeFormattingEditProvider(languageId, {
        autoFormatTriggerCharacters: [capability.firstTriggerCharacter, ...(capability.moreTriggerCharacter ?? [])],
        provideOnTypeFormattingEdits: async (model, position, ch, options, token: CancellationToken) => {
            const result = await client.request<TextEdit[] | null>('textDocument/onTypeFormatting', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
                ch,
                options: { tabSize: options.tabSize, insertSpaces: options.insertSpaces },
            })
            if (token.isCancellationRequested || !result) return []
            return toMonacoTextEdits(result)
        },
    })
}
