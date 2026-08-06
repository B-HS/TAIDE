import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { InlayHint } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled, markupContentToString } from '@shared/lib/lsp/protocol'
import { lspPositionToMonaco, monacoRangeToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

const INLAY_HINT_KIND_NAMES = ['Type', 'Parameter'] as const

const toMonacoInlayHint = (monaco: Monaco, hint: InlayHint) => ({
    position: lspPositionToMonaco(hint.position),
    label: typeof hint.label === 'string' ? hint.label : hint.label.map((part) => part.value).join(''),
    kind: hint.kind ? monaco.languages.InlayHintKind[INLAY_HINT_KIND_NAMES[hint.kind - 1]] : undefined,
    tooltip: markupContentToString(hint.tooltip),
    paddingLeft: hint.paddingLeft,
    paddingRight: hint.paddingRight,
})

export const registerInlayHints = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.inlayHintProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerInlayHintsProvider(languageId, {
        provideInlayHints: async (model, range) => {
            const result = await client.request<InlayHint[] | null>('textDocument/inlayHint', {
                textDocument: { uri: model.uri.toString() },
                range: monacoRangeToLsp(range),
            })
            return { hints: (result ?? []).map((hint) => toMonacoInlayHint(monaco, hint)), dispose: () => {} }
        },
    })
}
