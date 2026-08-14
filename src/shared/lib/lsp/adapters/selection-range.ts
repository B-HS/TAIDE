import type { CancellationToken } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { SelectionRange } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

export const flattenSelectionRangeChain = (selectionRange: SelectionRange | undefined | null) => {
    const chain: SelectionRange[] = []
    let current = selectionRange ?? undefined
    while (current) {
        chain.push(current)
        current = current.parent
    }
    return chain
}

export const toMonacoSelectionRangeChain = (selectionRange: SelectionRange | undefined | null) =>
    flattenSelectionRangeChain(selectionRange).map((entry) => ({ range: lspRangeToMonaco(entry.range) }))

export const registerSelectionRange = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.selectionRangeProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerSelectionRangeProvider(languageId, {
        provideSelectionRanges: async (model, positions, token: CancellationToken) => {
            const result = await client.request<(SelectionRange | null)[] | null>('textDocument/selectionRange', {
                textDocument: { uri: model.uri.toString() },
                positions: positions.map(monacoPositionToLsp),
            })
            if (token.isCancellationRequested || !result) return positions.map(() => [])
            return result.map(toMonacoSelectionRangeChain)
        },
    })
}
