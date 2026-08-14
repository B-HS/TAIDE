import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Location } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { preloadPeekModels } from '@shared/lib/lsp/peek-model-preload'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

const targetPathOf = (monaco: Monaco, location: Location) => {
    const uri = monaco.Uri.parse(location.uri)
    return uri.scheme === 'file' ? uri.fsPath : null
}

export const registerReferences = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.referencesProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerReferenceProvider(languageId, {
        provideReferences: async (model, position, context, token) => {
            const result = await client.request<Location[] | null>('textDocument/references', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
                context: { includeDeclaration: context.includeDeclaration },
            })
            if (token.isCancellationRequested || !result) return []

            const targetPaths = result.map((location) => targetPathOf(monaco, location)).filter((path): path is string => path !== null)
            await preloadPeekModels(monaco, targetPaths)
            if (token.isCancellationRequested) return []

            return result.map((location) => ({ uri: monaco.Uri.parse(location.uri), range: lspRangeToMonaco(location.range) }))
        },
    })
}
