import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { NOOP_DISPOSABLE } from '@shared/lib/lsp/noop-disposable'
import type { Location, LocationLink, ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { preloadPeekModels } from '@shared/lib/lsp/peek-model-preload'
import { lspLocationTargetPath, lspLocationToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

type LocationDisposable = ReturnType<Monaco['languages']['registerDefinitionProvider']>
type PositionLocationProvideFn = Parameters<Monaco['languages']['registerDefinitionProvider']>[1]['provideDefinition']

type PositionLocationAdapterConfig = {
    isSupported: (capabilities: ServerCapabilities) => boolean
    lspMethod: string
    register: (monaco: Monaco, languageId: string, provide: PositionLocationProvideFn) => LocationDisposable
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
            const targetPaths = items.map((item) => lspLocationTargetPath(monaco, item)).filter((path): path is string => path !== null)
            await preloadPeekModels(monaco, targetPaths)
            if (token.isCancellationRequested) return null

            return items.map((item) => lspLocationToMonaco(monaco, item))
        }

        return config.register(monaco, languageId, provide)
    }

export const registerDefinition = createLocationRequestAdapter({
    isSupported: (capabilities) => isCapabilityEnabled(capabilities.definitionProvider),
    lspMethod: 'textDocument/definition',
    register: (monaco, languageId, provide) => monaco.languages.registerDefinitionProvider(languageId, { provideDefinition: provide }),
})
