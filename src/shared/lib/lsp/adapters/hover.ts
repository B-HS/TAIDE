import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Hover } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled, markupContentToString } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

const toContentsValue = (contents: Hover['contents']) => {
    if (Array.isArray(contents)) return contents.map((part) => markupContentToString(part) ?? '').join('\n\n')
    return markupContentToString(contents) ?? ''
}

export const registerHover = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.hoverProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerHoverProvider(languageId, {
        provideHover: async (model, position) => {
            const result = await client.request<Hover | null>('textDocument/hover', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
            })
            if (!result) return null
            return {
                contents: [{ value: toContentsValue(result.contents) }],
                range: result.range ? lspRangeToMonaco(result.range) : undefined,
            }
        },
    })
}
