import { createLocationRequestAdapter } from '@shared/lib/lsp/adapters/definition'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'

export const registerImplementation = createLocationRequestAdapter({
    isSupported: (capabilities) => isCapabilityEnabled(capabilities.implementationProvider),
    lspMethod: 'textDocument/implementation',
    register: (monaco, languageId, provide) => monaco.languages.registerImplementationProvider(languageId, { provideImplementation: provide }),
})
