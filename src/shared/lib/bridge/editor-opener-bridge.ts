import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

export type OpenFileFromEditorRequest = { path: string; line: number; column: number }

const openFileFromEditorBridge = createFireAndForgetBridge<OpenFileFromEditorRequest>()

export const requestOpenFileFromEditor = openFileFromEditorBridge.publish
export const subscribeOpenFileFromEditor = openFileFromEditorBridge.subscribe

type RegisterEditorOpener = Monaco['editor']['registerEditorOpener']
type CodeEditorOpener = Parameters<RegisterEditorOpener>[0]
type OpenCodeEditorFn = CodeEditorOpener['openCodeEditor']
type SelectionOrPosition = Parameters<OpenCodeEditorFn>[2]

const FILE_SCHEME = 'file'
const UNTITLED_SCHEME = 'untitled'
const DEFAULT_LINE = 1
const DEFAULT_COLUMN = 1

const toLineColumn = (selectionOrPosition: SelectionOrPosition) => {
    if (!selectionOrPosition) return { line: DEFAULT_LINE, column: DEFAULT_COLUMN }
    if ('startLineNumber' in selectionOrPosition) return { line: selectionOrPosition.startLineNumber, column: selectionOrPosition.startColumn }
    return { line: selectionOrPosition.lineNumber, column: selectionOrPosition.column }
}

/**
 * Registers monaco's cross-file navigation hook (`registerEditorOpener`), which monaco calls
 * whenever go-to-definition/references/F8/peek needs to open a resource other than the current
 * model. Standalone monaco has no built-in way to open a different TAIDE tab, so without this the
 * request silently no-ops. Same-model navigation (the overwhelmingly common case — jumping within
 * the file already open in `source`) is intentionally left to monaco's own default handling by
 * returning `false`, both for correctness (monaco resolves it against the exact source editor
 * instance, which matters when the same file is split across panes) and to avoid an unnecessary
 * openTab round-trip on every same-file jump.
 *
 * Call once during app bootstrap; the returned disposable unregisters the opener.
 */
export const registerLspEditorOpener = (monaco: Monaco) =>
    monaco.editor.registerEditorOpener({
        openCodeEditor: (source, resource, selectionOrPosition) => {
            if (resource.toString() === source.getModel()?.uri.toString()) return false

            if (resource.scheme === UNTITLED_SCHEME) {
                const targetEditor = monaco.editor.getEditors().find((editor) => editor.getModel()?.uri.toString() === resource.toString())
                if (!targetEditor) return false
                const { line, column } = toLineColumn(selectionOrPosition)
                targetEditor.setPosition({ lineNumber: line, column })
                targetEditor.revealPositionInCenter({ lineNumber: line, column })
                targetEditor.focus()
                return true
            }

            if (resource.scheme !== FILE_SCHEME) return false
            if (!openFileFromEditorBridge.hasSubscribers()) return false

            const { line, column } = toLineColumn(selectionOrPosition)
            requestOpenFileFromEditor({ path: resource.fsPath, line, column })
            return true
        },
    })
