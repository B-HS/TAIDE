import type { CancellationToken, languages } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { NOOP_DISPOSABLE } from '@shared/lib/lsp/noop-disposable'
import type { DocumentSymbol, SymbolInformation } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco } from '@shared/lib/lsp/position'

const SYMBOL_KIND_NAMES = [
    'File',
    'Module',
    'Namespace',
    'Package',
    'Class',
    'Method',
    'Property',
    'Field',
    'Constructor',
    'Enum',
    'Interface',
    'Function',
    'Variable',
    'Constant',
    'String',
    'Number',
    'Boolean',
    'Array',
    'Object',
    'Key',
    'Null',
    'EnumMember',
    'Struct',
    'Event',
    'Operator',
    'TypeParameter',
] as const

const toMonacoKind = (monaco: Monaco, kind: number) => {
    const name = SYMBOL_KIND_NAMES[kind - 1]
    return name ? monaco.languages.SymbolKind[name] : monaco.languages.SymbolKind.Variable
}

const isDocumentSymbol = (symbol: DocumentSymbol | SymbolInformation): symbol is DocumentSymbol => 'range' in symbol

const toMonacoDocumentSymbol = (monaco: Monaco, symbol: DocumentSymbol | SymbolInformation): languages.DocumentSymbol => {
    if (isDocumentSymbol(symbol)) {
        return {
            name: symbol.name,
            detail: symbol.detail ?? '',
            kind: toMonacoKind(monaco, symbol.kind),
            tags: [],
            range: lspRangeToMonaco(symbol.range),
            selectionRange: lspRangeToMonaco(symbol.selectionRange),
            children: symbol.children?.map((child) => toMonacoDocumentSymbol(monaco, child)),
        }
    }
    const range = lspRangeToMonaco(symbol.location.range)
    return { name: symbol.name, detail: '', kind: toMonacoKind(monaco, symbol.kind), tags: [], range, selectionRange: range }
}

/**
 * `token` is optional because this function is also called directly by the outline panel
 * (`widgets/outline-panel`) for a one-off request outside monaco's provider lifecycle, where no
 * `CancellationToken` exists. Only the monaco provider path below supplies one.
 */
export const requestDocumentSymbols = async (monaco: Monaco, client: LspClient, uri: string, token?: CancellationToken) => {
    const result = await client.request<(DocumentSymbol | SymbolInformation)[] | null>('textDocument/documentSymbol', {
        textDocument: { uri },
    })
    if (token?.isCancellationRequested) return []
    return (result ?? []).map((symbol) => toMonacoDocumentSymbol(monaco, symbol))
}

export const registerDocumentSymbol = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.documentSymbolProvider))) return NOOP_DISPOSABLE

    return monaco.languages.registerDocumentSymbolProvider(languageId, {
        provideDocumentSymbols: (model, token) => requestDocumentSymbols(monaco, client, model.uri.toString(), token),
    })
}
