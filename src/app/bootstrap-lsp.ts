import { registerLspEditorOpener } from '@shared/lib/editor-opener-bridge'
import { registerLspClientNavigationCommands } from '@shared/lib/lsp/command-relay'
import { monaco } from '@shared/lib/monaco/setup'

registerLspEditorOpener(monaco)
registerLspClientNavigationCommands(monaco)
