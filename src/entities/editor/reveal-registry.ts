import { monaco } from '@shared/lib/monaco/setup'

type RevealTarget = { line: number; column: number }

const pendingReveals = new Map<string, RevealTarget>()

const toKey = (path: string) => monaco.Uri.file(path).toString()

const findEditorForPath = (path: string) => {
    const key = toKey(path)
    return monaco.editor.getEditors().find((editor) => editor.getModel()?.uri.toString() === key) ?? null
}

const applyReveal = (editor: monaco.editor.ICodeEditor, target: RevealTarget) => {
    const position = { lineNumber: target.line, column: target.column }
    editor.setPosition(position)
    editor.revealPositionInCenter(position)
    editor.focus()
}

export const requestReveal = (path: string, line: number, column = 1) => {
    const editor = findEditorForPath(path)
    if (editor) {
        applyReveal(editor, { line, column })
        return
    }
    pendingReveals.set(toKey(path), { line, column })
}

export const consumePendingReveal = (path: string, editor: monaco.editor.ICodeEditor) => {
    const key = toKey(path)
    const target = pendingReveals.get(key)
    if (!target) return
    pendingReveals.delete(key)
    applyReveal(editor, target)
}
