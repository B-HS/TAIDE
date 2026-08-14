import type { CancellationToken } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { SignatureHelp } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled, markupContentToString } from '@shared/lib/lsp/protocol'
import { monacoPositionToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

export const registerSignatureHelp = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.signatureHelpProvider))) return NOOP_DISPOSABLE

    const capability = client.getCapabilities()?.signatureHelpProvider

    return monaco.languages.registerSignatureHelpProvider(languageId, {
        signatureHelpTriggerCharacters: capability?.triggerCharacters ?? [],
        signatureHelpRetriggerCharacters: capability?.retriggerCharacters ?? [],
        provideSignatureHelp: async (model, position, token: CancellationToken) => {
            const result = await client.request<SignatureHelp | null>('textDocument/signatureHelp', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
            })
            if (token.isCancellationRequested || !result) return null
            return {
                value: {
                    signatures: result.signatures.map((signature) => ({
                        label: signature.label,
                        documentation: markupContentToString(signature.documentation),
                        parameters: (signature.parameters ?? []).map((parameter) => ({
                            label: parameter.label,
                            documentation: markupContentToString(parameter.documentation),
                        })),
                    })),
                    activeSignature: result.activeSignature ?? 0,
                    activeParameter: result.activeParameter ?? 0,
                },
                dispose: () => {},
            }
        },
    })
}
