import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { NOOP_DISPOSABLE } from '@shared/lib/lsp/noop-disposable'
import type { LspRange, PrepareRenameResult, WorkspaceEdit } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoPositionToLsp } from '@shared/lib/lsp/position'
import { applyWorkspaceEdit } from '@shared/lib/lsp/workspace-edit-applier'
import { i18next } from '@shared/i18n/i18n'

export const registerRename = (monaco: Monaco, client: LspClient, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.renameProvider))) return NOOP_DISPOSABLE

    const supportsPrepare = client.supports(
        (capabilities) => typeof capabilities.renameProvider === 'object' && capabilities.renameProvider.prepareProvider === true,
    )

    return monaco.languages.registerRenameProvider(languageId, {
        /**
         * Applies the rename itself via {@link applyWorkspaceEdit} (open models get
         * `pushEditOperations`, unopened files go through the file IPC, resource operations are
         * supported) and always returns an empty edit list — monaco's own `bulkEditService.apply`
         * on an *empty* `WorkspaceEdit` is a guaranteed no-op, so this sidesteps
         * `StandaloneBulkEditService` throwing for files it has no open model for (the rename
         * limitation this unifies away). A failed apply surfaces through `rejectReason`, the same
         * channel `resolveRenameLocation` below already uses for "no result" cases.
         */
        provideRenameEdits: async (model, position, newName) => {
            const result = await client.request<WorkspaceEdit | null>('textDocument/rename', {
                textDocument: { uri: model.uri.toString() },
                position: monacoPositionToLsp(position),
                newName,
            })
            if (!result) return { edits: [] }
            const applyResult = await applyWorkspaceEdit(monaco, result, undefined, { getDocumentVersion: client.getDocumentVersion })
            if (!applyResult.applied) return { edits: [], rejectReason: applyResult.failureReason ?? i18next.t('editor.workspaceEditApplyFailed') }
            return { edits: [] }
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
                  if (!result) return { range: fallbackRange, text: word?.word ?? '', rejectReason: i18next.t('editor.renameUnavailable') }
                  const range: LspRange = 'range' in result ? result.range : result
                  const text = 'placeholder' in result ? result.placeholder : (word?.word ?? '')
                  return { range: lspRangeToMonaco(range), text }
              }
            : undefined,
    })
}
