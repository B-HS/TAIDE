import type { CancellationToken, languages } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'

const NOOP_DISPOSABLE = { dispose: () => {} }

export const FOLDING_RANGE_CLIENT_LIMIT = 5000

export type LspFoldingRange = { startLine: number; endLine: number; kind?: string }

/**
 * Converts LSP's 0-based `startLine`/`endLine` to monaco's 1-based `start`/`end`. Kept separate
 * from {@link toMonacoFoldingRange} (which additionally needs a live `Monaco` instance for
 * `FoldingRangeKind.fromValue`) so the line-number arithmetic is unit-testable without loading
 * the real monaco-editor runtime.
 */
export const toMonacoFoldingRangeLines = (range: Pick<LspFoldingRange, 'startLine' | 'endLine'>) => ({
    start: range.startLine + 1,
    end: range.endLine + 1,
})

const toMonacoFoldingRange = (monaco: Monaco, range: LspFoldingRange): languages.FoldingRange => ({
    ...toMonacoFoldingRangeLines(range),
    kind: range.kind ? monaco.languages.FoldingRangeKind.fromValue(range.kind) : undefined,
})

export const registerFoldingRange = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.foldingRangeProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerFoldingRangeProvider(languageId, {
        provideFoldingRanges: async (model, _context, token: CancellationToken) => {
            const result = await client.request<LspFoldingRange[] | null>('textDocument/foldingRange', {
                textDocument: { uri: model.uri.toString() },
            })
            if (token.isCancellationRequested || !result) return []
            return result.slice(0, FOLDING_RANGE_CLIENT_LIMIT).map((range) => toMonacoFoldingRange(monaco, range))
        },
    })
}
