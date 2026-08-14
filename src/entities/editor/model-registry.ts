import { cancelPeekModelDispose } from '@shared/lib/lsp/peek-model-preload'
import { monaco } from '@shared/lib/monaco/setup'

type ModelEntry = {
    model: monaco.editor.ITextModel
    viewState: monaco.editor.ICodeEditorViewState | null
}

const registry = new Map<string, ModelEntry>()

const UNTITLED_URI_PREFIX = 'untitled:'
const WINDOWS_DRIVE_PATH_PATTERN = /^[a-zA-Z]:[\\/]/

const isAbsoluteFilePath = (path: string) => path.startsWith('/') || WINDOWS_DRIVE_PATH_PATTERN.test(path)

const isUntitledPath = (path: string) => !isAbsoluteFilePath(path) && path.startsWith(UNTITLED_URI_PREFIX)

const toUri = (path: string) => (isUntitledPath(path) ? monaco.Uri.parse(path) : monaco.Uri.file(path))

const toKey = (path: string) => toUri(path).toString()

export const toUntitledModelPath = (tabId: string) => `${UNTITLED_URI_PREFIX}${tabId}`

export const getModel = (path: string) => registry.get(toKey(path))?.model

/**
 * Returns the model registered for `path`, adopting one already present in monaco's own model
 * service (an orphan not yet tracked by this registry — `peek-model-preload.ts` creates real
 * models at the real file uri via `monaco.editor.createModel` directly, so Peek can preview a
 * file that has no open tab) before falling back to creating a fresh model. `content`/`languageId`
 * are only used in that fallback case.
 *
 * Adopting an orphan cancels its peek-preload auto-dispose timer (`cancelPeekModelDispose`) —
 * once a real tab takes over a model's lifecycle, the bounded TTL that exists only to reclaim
 * unused peek previews must no longer be able to dispose it out from under the tab. A cached
 * entry whose model has since been disposed by something outside this registry's control is
 * dropped and rebuilt instead of being handed back disposed (self-healing, defensive).
 */
export const getOrCreateModel = (path: string, content: string, languageId: string) => {
    const uri = toUri(path)
    const key = uri.toString()
    const existing = registry.get(key)
    if (existing && !existing.model.isDisposed()) return existing.model
    if (existing) registry.delete(key)

    const orphanModel = monaco.editor.getModel(uri)
    if (orphanModel) {
        cancelPeekModelDispose(path)
        registry.set(key, { model: orphanModel, viewState: null })
        return orphanModel
    }

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
