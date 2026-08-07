import { monaco } from '@shared/lib/monaco/setup'

type ModelEntry = {
    model: monaco.editor.ITextModel
    viewState: monaco.editor.ICodeEditorViewState | null
}

const registry = new Map<string, ModelEntry>()

const UNTITLED_URI_PREFIX = 'untitled:'
const WINDOWS_DRIVE_PATH_PATTERN = /^[a-zA-Z]:[\\/]/

export class ModelAlreadyExistsError extends Error {
    constructor(path: string) {
        super(`model already exists for path: ${path}`)
        this.name = 'ModelAlreadyExistsError'
    }
}

const isAbsoluteFilePath = (path: string) => path.startsWith('/') || WINDOWS_DRIVE_PATH_PATTERN.test(path)

const isUntitledPath = (path: string) => !isAbsoluteFilePath(path) && path.startsWith(UNTITLED_URI_PREFIX)

const toUri = (path: string) => (isUntitledPath(path) ? monaco.Uri.parse(path) : monaco.Uri.file(path))

const toKey = (path: string) => toUri(path).toString()

export const toUntitledModelPath = (tabId: string) => `${UNTITLED_URI_PREFIX}${tabId}`

export const getModel = (path: string) => registry.get(toKey(path))?.model

export const createModel = (path: string, content: string, languageId: string) => {
    const uri = toUri(path)
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
