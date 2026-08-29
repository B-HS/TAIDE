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

/**
 * Moves the model registered for `from` over to `to` after a rename, keeping the buffer (unsaved
 * edits included), the language and the saved view state, and re-pointing every editor currently
 * displaying it at the new model *before* the old one is disposed.
 *
 * A model cannot follow a rename in place: `ITextModel.uri` is immutable, and the uri is what the
 * LSP adapters, the diagnostics markers and the peek/definition surfaces all address a document by —
 * leaving it at the old path would keep sending edits and requests for a file that no longer exists.
 * So the move is create-new + dispose-old, which costs this document's **undo history** (monaco has
 * no uri-move API that preserves it) — the same trade `useCloseTab`'s dispose makes, recorded in
 * contract §3 S8.
 *
 * The editor re-point is what makes the dispose safe *and* seamless. Monaco already self-heals
 * (`CodeEditorWidget` subscribes to `model.onWillDispose` and detaches to a null model), but that
 * would blank the pane until React re-rendered it onto the new path; swapping the model here instead
 * keeps the same text on screen, and restoring each editor's own view state keeps its cursor and
 * scroll position across the rename.
 *
 * A stale model already sitting at the destination uri (a tab for a path that was deleted and whose
 * model outlived it, or a peek preload) is disposed first — `monaco.editor.createModel` throws on a
 * duplicate uri, which would otherwise turn a rename into a hard failure.
 */
export const retargetModel = (from: string, to: string) => {
    const fromKey = toKey(from)
    const entry = registry.get(fromKey)
    if (!entry) return

    registry.delete(fromKey)
    if (entry.model.isDisposed()) return

    const targetUri = toUri(to)
    const targetKey = targetUri.toString()
    const staleTarget = registry.get(targetKey)?.model ?? monaco.editor.getModel(targetUri)
    registry.delete(targetKey)
    cancelPeekModelDispose(to)

    const displacedEditors = monaco.editor
        .getEditors()
        .filter((editor) => editor.getModel() === entry.model || (!!staleTarget && editor.getModel() === staleTarget))
        .map((editor) => ({ editor, viewState: editor.saveViewState() }))

    if (staleTarget && !staleTarget.isDisposed()) staleTarget.dispose()

    const model = monaco.editor.createModel(entry.model.getValue(), entry.model.getLanguageId(), targetUri)
    registry.set(targetKey, { model, viewState: entry.viewState })

    for (const { editor, viewState } of displacedEditors) {
        editor.setModel(model)
        if (viewState) editor.restoreViewState(viewState)
    }
    entry.model.dispose()
}

/**
 * Re-applies `languageId` to the model registered for `path` when it no longer matches — the tail of
 * a rename that changed the extension (`notes.txt` → `notes.ts`). {@link retargetModel} carries the
 * old language over because only the backend's re-read of the new path knows the new one, and
 * {@link getOrCreateModel} never re-languages a model it merely hands back, so without this the
 * moved buffer would keep the old grammar until the tab was closed and reopened.
 */
export const applyModelLanguage = (path: string, languageId: string) => {
    const entry = registry.get(toKey(path))
    if (!entry || entry.model.isDisposed() || entry.model.getLanguageId() === languageId) return

    monaco.editor.setModelLanguage(entry.model, languageId)
}

/**
 * Calls `listener` whenever the buffer at `path` changes, for as long as the returned unsubscribe
 * has not been called — including across a model that does not exist yet at subscribe time, or one
 * disposed and recreated under the same uri ({@link retargetModel}, a tab closed and reopened, a
 * peek preload promoted to a real tab). Consumers that must re-derive something from the buffer's
 * *content* (the document symbol re-request behind the outline/breadcrumb surfaces, audit §4-B B12)
 * cannot use `getModel(path)?.onDidChangeContent` directly for exactly that reason: the model they
 * would find at effect time is not necessarily the one the pane ends up editing, and a subscription
 * to a disposed model is silently dead.
 *
 * Reads through `monaco.editor.getModel`/`onDidCreateModel` rather than this registry's own map so
 * an orphan model (`peek-model-preload.ts` creates real models at the real file uri) is observed
 * too — the registry only learns about those when a tab adopts one.
 */
export const subscribeModelContentChange = (path: string, listener: () => void) => {
    const uri = toUri(path)
    const key = uri.toString()
    let contentSubscription: { dispose: () => void } | null = null

    const attach = (model: monaco.editor.ITextModel) => {
        contentSubscription?.dispose()
        contentSubscription = model.onDidChangeContent(() => listener())
    }

    const existingModel = monaco.editor.getModel(uri)
    if (existingModel) attach(existingModel)

    const createSubscription = monaco.editor.onDidCreateModel((model) => {
        if (model.uri.toString() !== key) return
        attach(model)
    })

    return () => {
        createSubscription.dispose()
        contentSubscription?.dispose()
        contentSubscription = null
    }
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

/**
 * Models currently being rewritten by {@link applyExternalContent}. `ITextModel.setValue` notifies
 * `onDidChangeModelContent` synchronously, so without this the pane's own "the model now matches
 * disk / the restored mirror" write comes back through `CodeEditor`'s change subscription
 * indistinguishable from a keystroke: the tab flips dirty with content identical to what it just
 * adopted, and the hot-exit mirror debounce that transition arms then persists that same content as
 * "unsaved recovery data". Every caller of `applyExternalContent` already sets whatever draft/dirty
 * state its own write implies (`use-editor-file-persistence.ts`'s mirror restore marks dirty,
 * `handleViewDisk` marks clean), so the echoed change event carries no information — only damage.
 */
const modelsApplyingExternalContent = new WeakSet<monaco.editor.ITextModel>()

/** Whether `model`'s in-flight content change originates from {@link applyExternalContent} rather than from an edit. */
export const isApplyingExternalContentTo = (model: monaco.editor.ITextModel) => modelsApplyingExternalContent.has(model)

export const applyExternalContent = (path: string, content: string, editor: monaco.editor.ICodeEditor | null) => {
    const entry = registry.get(toKey(path))
    if (!entry || entry.model.getValue() === content) return

    const viewState = editor?.saveViewState() ?? null
    modelsApplyingExternalContent.add(entry.model)
    try {
        entry.model.setValue(content)
    } finally {
        modelsApplyingExternalContent.delete(entry.model)
    }
    if (editor && viewState) editor.restoreViewState(viewState)
}
