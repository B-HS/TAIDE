import { monaco } from '@shared/lib/monaco/setup'

type ModelEntry = {
    model: monaco.editor.ITextModel
    viewState: monaco.editor.ICodeEditorViewState | null
}

const registry = new Map<string, ModelEntry>()

export class ModelAlreadyExistsError extends Error {
    constructor(path: string) {
        super(`model already exists for path: ${path}`)
        this.name = 'ModelAlreadyExistsError'
    }
}

const toKey = (path: string) => monaco.Uri.file(path).toString()

export const getModel = (path: string) => registry.get(toKey(path))?.model

export const createModel = (path: string, content: string, languageId: string) => {
    const uri = monaco.Uri.file(path)
    const key = uri.toString()
    if (registry.has(key) || monaco.editor.getModel(uri)) throw new ModelAlreadyExistsError(path)

    const model = monaco.editor.createModel(content, languageId, uri)
    registry.set(key, { model, viewState: null })
    return model
}

export const disposeModel = (path: string) => {
    const key = toKey(path)
    const entry = registry.get(key)
    if (!entry) return

    registry.delete(key)
    entry.model.dispose()
}

export const saveViewState = (path: string, editor: monaco.editor.ICodeEditor) => {
    const entry = registry.get(toKey(path))
    if (!entry) return

    entry.viewState = editor.saveViewState()
}

export const restoreViewState = (path: string, editor: monaco.editor.ICodeEditor) => {
    const entry = registry.get(toKey(path))
    if (!entry?.viewState) return

    editor.restoreViewState(entry.viewState)
}

export const applyExternalContent = (path: string, content: string, editor: monaco.editor.ICodeEditor | null) => {
    const entry = registry.get(toKey(path))
    if (!entry || entry.model.getValue() === content) return

    const viewState = editor?.saveViewState() ?? null
    entry.model.setValue(content)
    if (editor && viewState) editor.restoreViewState(viewState)
}
