import { createLocationRequestAdapter } from '@shared/lib/lsp/adapters/definition'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'

export const registerDeclaration = createLocationRequestAdapter({
    isSupported: (capabilities) => isCapabilityEnabled(capabilities.declarationProvider),
    lspMethod: 'textDocument/declaration',
    register: (monaco, languageId, provide) => monaco.languages.registerDeclarationProvider(languageId, { provideDeclaration: provide }),
})
