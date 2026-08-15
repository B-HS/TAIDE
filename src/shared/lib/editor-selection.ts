import type { editor } from 'monaco-editor'

/**
 * Resolves the text a "run in terminal" action should send for `editorInstance` — the current
 * selection when non-empty, otherwise the full content of the line the cursor sits on (matching
 * VS Code's "Run Selected Text" fallback). Returns `null` when the editor has no model or cursor
 * position to read from.
 */
export const resolveSelectedTextOrCurrentLine = (editorInstance: editor.ICodeEditor): string | null => {
    const model = editorInstance.getModel()
    if (!model) return null

    const selection = editorInstance.getSelection()
    if (selection && !selection.isEmpty()) return model.getValueInRange(selection)

    const line = editorInstance.getPosition()?.lineNumber
    if (!line) return null
    return model.getLineContent(line)
}
