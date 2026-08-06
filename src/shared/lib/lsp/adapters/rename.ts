import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { LspRange, PrepareRenameResult, WorkspaceEdit } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

const toResourceEdits = (monaco: Monaco, edit: WorkspaceEdit) => {
    const changes = edit.changes ?? {}
    return Object.entries(changes).flatMap(([uri, edits]) =>
        edits.map((textEdit) => ({
            resource: monaco.Uri.parse(uri),
            textEdit: { range: lspRangeToMonaco(textEdit.range), text: textEdit.newText },
            versionId: undefined,
        })),
    )
}

export const registerRename = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.renameProvider))) return NOOP_DISPOSABLE

    const supportsPrepare = client.supports(
        (capabilities) => typeof capabilities.renameProvider === 'object' && capabilities.renameProvider.prepareProvider === true,
    )

    return monaco.languages.registerRenameProvider(languageId, {
        provideRenameEdits: async (model, position, newName) => {
            const result = await client.request<WorkspaceEdit | null>('textDocument/rename', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
                newName,
            })
            if (!result) return { edits: [] }
            return { edits: toResourceEdits(monaco, result) }
        },
        resolveRenameLocation: supportsPrepare
            ? async (model, position) => {
                  const result = await client.request<PrepareRenameResult | null>('textDocument/prepareRename', {
                      textDocument: { uri: model.uri.toString() },
                      position: monacoPositionToLsp(position),
                  })
                  const word = model.getWordAtPosition(position)
                  const fallbackRange = word
                      ? new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn)
                      : monaco.Range.fromPositions(position, position)
                  if (!result) return { range: fallbackRange, text: word?.word ?? '', rejectReason: '이름을 바꿀 수 없는 위치입니다' }
                  const range: LspRange = 'range' in result ? result.range : result
                  const text = 'placeholder' in result ? result.placeholder : (word?.word ?? '')
                  return { range: lspRangeToMonaco(range), text }
              }
            : undefined,
    })
}
