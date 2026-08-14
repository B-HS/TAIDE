import { createLocationRequestAdapter } from '@shared/lib/lsp/adapters/definition'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'

export const registerTypeDefinition = createLocationRequestAdapter({
    isSupported: (capabilities) => isCapabilityEnabled(capabilities.typeDefinitionProvider),
    lspMethod: 'textDocument/typeDefinition',
    register: (monaco, languageId, provide) => monaco.languages.registerTypeDefinitionProvider(languageId, { provideTypeDefinition: provide }),
})
