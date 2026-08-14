import { registerLspEditorOpener } from '@shared/lib/editor-opener-bridge'
import { registerLspClientNavigationCommands } from '@shared/lib/lsp/command-relay'
import { registerWorkspaceApplyEditHandler } from '@shared/lib/lsp/workspace-edit-apply-handler'
import { monaco } from '@shared/lib/monaco/setup'

registerLspEditorOpener(monaco)
registerWorkspaceApplyEditHandler(monaco)
registerLspClientNavigationCommands(monaco)
