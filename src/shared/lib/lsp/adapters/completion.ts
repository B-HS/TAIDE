import type { CancellationToken, IRange } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { NOOP_DISPOSABLE } from '@shared/lib/lsp/noop-disposable'
import type { CompletionItem, CompletionList } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled, markupContentToString } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const COMPLETION_ITEM_KIND_NAMES = [
    'Text',
    'Method',
    'Function',
    'Constructor',
    'Field',
    'Variable',
    'Class',
    'Interface',
    'Module',
    'Property',
    'Unit',
    'Value',
    'Enum',
    'Keyword',
    'Snippet',
    'Color',
    'File',
    'Reference',
    'Folder',
    'EnumMember',
    'Constant',
    'Struct',
    'Event',
    'Operator',
    'TypeParameter',
] as const

const SNIPPET_INSERT_TEXT_FORMAT = 2

export const registerCompletion = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.completionProvider))) return NOOP_DISPOSABLE

    const triggerCharacters = client.getCapabilities()?.completionProvider?.triggerCharacters ?? []

    const toMonacoKind = (kind: number | undefined) => {
        const name = kind === undefined ? undefined : COMPLETION_ITEM_KIND_NAMES[kind - 1]
        return name ? monaco.languages.CompletionItemKind[name] : monaco.languages.CompletionItemKind.Text
    }

    const toMonacoItem = (item: CompletionItem, defaultRange: IRange) => ({
        label: item.label,
        kind: toMonacoKind(item.kind),
        detail: item.detail,
        documentation: markupContentToString(item.documentation),
        insertText: item.textEdit?.newText ?? item.insertText ?? item.label,
        insertTextRules:
            item.insertTextFormat === SNIPPET_INSERT_TEXT_FORMAT ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
        sortText: item.sortText,
        filterText: item.filterText,
        range: item.textEdit ? lspRangeToMonaco(item.textEdit.range) : defaultRange,
    })

    return monaco.languages.registerCompletionItemProvider(languageId, {
        triggerCharacters,
        provideCompletionItems: async (model, position, _context, token: CancellationToken) => {
            const result = await client.request<CompletionItem[] | CompletionList | null>('textDocument/completion', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
            })
            if (token.isCancellationRequested) return { suggestions: [] }
            const items = result === null ? [] : Array.isArray(result) ? result : result.items
            const word = model.getWordUntilPosition(position)
            const defaultRange = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn)
            return { suggestions: items.map((item) => toMonacoItem(item, defaultRange)) }
        },
    })
}
