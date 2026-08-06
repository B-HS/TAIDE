import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Location, LocationLink } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

const toMonacoLocation = (monaco: Monaco, item: Location | LocationLink) => {
    if ('targetUri' in item) return { uri: monaco.Uri.parse(item.targetUri), range: lspRangeToMonaco(item.targetRange) }
    return { uri: monaco.Uri.parse(item.uri), range: lspRangeToMonaco(item.range) }
}

export const registerDefinition = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.definitionProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerDefinitionProvider(languageId, {
        provideDefinition: async (model, position) => {
            const result = await client.request<Location | Location[] | LocationLink[] | null>('textDocument/definition', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
            })
            if (!result) return null
            const items = Array.isArray(result) ? result : [result]
            return items.map((item) => toMonacoLocation(monaco, item))
        },
    })
}
