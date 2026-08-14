import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Location, LocationLink, ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { preloadPeekModels } from '@shared/lib/lsp/peek-model-preload'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

type LocationDisposable = ReturnType<Monaco['languages']['registerDefinitionProvider']>
type PositionLocationProvideFn = Parameters<Monaco['languages']['registerDefinitionProvider']>[1]['provideDefinition']

type PositionLocationAdapterConfig = {
    isSupported: (capabilities: ServerCapabilities) => boolean
    lspMethod: string
    register: (monaco: Monaco, languageId: string, provide: PositionLocationProvideFn) => LocationDisposable
}

/**
 * A `LocationLink`'s `targetRange` spans the whole declaration (doc comments, attributes and all);
 * `targetSelectionRange` is the precise identifier span monaco uses for the cursor position and
 * Peek highlight (LSP 3.17 `LocationLink`; monaco's own `isLocationLink`/`goToLocations` fall back
 * to `range` — i.e. `targetRange` — whenever `targetSelectionRange` is absent). Dropping it here
 * used to land F12/Peek on the declaration's doc comment instead of the symbol itself.
 */
const toMonacoLocation = (monaco: Monaco, item: Location | LocationLink) => {
    if ('targetUri' in item)
        return {
            uri: monaco.Uri.parse(item.targetUri),
            range: lspRangeToMonaco(item.targetRange),
            targetSelectionRange: lspRangeToMonaco(item.targetSelectionRange),
            ...(item.originSelectionRange ? { originSelectionRange: lspRangeToMonaco(item.originSelectionRange) } : {}),
        }
    return { uri: monaco.Uri.parse(item.uri), range: lspRangeToMonaco(item.range) }
}

const targetPathOf = (monaco: Monaco, item: Location | LocationLink) => {
    const uri = monaco.Uri.parse('targetUri' in item ? item.targetUri : item.uri)
    return uri.scheme === 'file' ? uri.fsPath : null
}

/**
 * Builds a `register*` adapter for LSP requests that resolve a symbol at a position to one or more
 * locations (definition/implementation/typeDefinition/declaration all share this exact shape). Only
 * the capability predicate, LSP method, and monaco registration call differ between them.
 */
export const createLocationRequestAdapter =
    (config: PositionLocationAdapterConfig) =>
    (monaco: Monaco, client: LspClient, languageId: string): LocationDisposable => {
        if (!client.supports(config.isSupported)) return NOOP_DISPOSABLE

        const provide: PositionLocationProvideFn = async (model, position, token) => {
            const result = await client.request<Location | Location[] | LocationLink[] | null>(config.lspMethod, {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
            })
            if (token.isCancellationRequested || !result) return null

            const items = Array.isArray(result) ? result : [result]
            const targetPaths = items.map((item) => targetPathOf(monaco, item)).filter((path): path is string => path !== null)
            await preloadPeekModels(monaco, targetPaths)
            if (token.isCancellationRequested) return null

            return items.map((item) => toMonacoLocation(monaco, item))
        }

        return config.register(monaco, languageId, provide)
    }

export const registerDefinition = createLocationRequestAdapter({
    isSupported: (capabilities) => isCapabilityEnabled(capabilities.definitionProvider),
    lspMethod: 'textDocument/definition',
    register: (monaco, languageId, provide) => monaco.languages.registerDefinitionProvider(languageId, { provideDefinition: provide }),
})
