/**
 * Tracks monaco file-path models `workspace-edit-applier.ts` edited via `pushEditOperations` while
 * no mounted `EditorPane` was watching them — a background tab in another pane, or a tab that
 * simply isn't the active tab in its pane right now. `entities/editor/model-registry.ts` never
 * disposes a file-path model once created, so it keeps existing (and keeps being treated as an
 * "open model" by the applier) long after the `EditorPane` that first opened it stops being
 * rendered — `widgets/editor-area/pane-node-view.tsx` only mounts the active tab's `EditorPane`.
 *
 * Nothing else needs to mark a model dirty through this module: a currently-mounted `EditorPane`
 * tracks its own dirty state directly off `onDidChangeModelContent` well before an edit could reach
 * here. This exists purely so `EditorPane`'s tab-activation sync effect (which decides whether to
 * overwrite a model with the last-known synced disk content) can tell "this model simply hasn't
 * diverged from disk yet" apart from "an edit already landed on this model while nothing was
 * watching it" — without this, that effect could not distinguish the two and would silently
 * discard the edit by refreshing the model from disk content the edit never touched.
 */
const externallyDirtyPaths = new Set<string>()

type ModelEditedExternallyListener = (path: string) => void

const externalEditListeners = new Set<ModelEditedExternallyListener>()

/**
 * Subscribes to the same event {@link markModelDirtyExternally} records, for consumers that must
 * react to it *now* rather than when the tab is next activated — specifically the tab bar's dirty
 * dot, which `layout_set_dirty` owns on the Rust side and which no unmounted `EditorPane` can raise.
 *
 * A bridge rather than a direct call because the only producer lives in `shared/lib/lsp`
 * (`workspace-edit-applier.ts`) while the only consumer is a widget holding a layout mutation
 * (`widgets/editor-pane/use-editor-lsp-integration.ts`), and `shared` cannot import `entities`
 * (fsd.md §2) — the same reason the applier reaches file IPC through `@shared/api/bindings` instead
 * of `@entities/file/file.ipc`. Going through the widget's `useSetTabDirty` (not a raw
 * `commands.layoutSetDirty`) is what keeps the layout query cache — and therefore the dot actually
 * on screen — in step with the backend flag.
 *
 * A `Set` (every listener notified), not a single slot: the main window mounts one `ProjectShell`
 * per shell slot, each with its own project and its own `EditorPane`s, so several listeners can be
 * live at once and each one only recognises paths belonging to its own project's layout. Two panes
 * of the *same* project both firing for one edit is harmless — setting an already-set dirty flag is
 * idempotent, and the listener skips tabs the cache already reports as dirty.
 */
export const onModelEditedExternally = (listener: ModelEditedExternallyListener) => {
    externalEditListeners.add(listener)
    return () => {
        externalEditListeners.delete(listener)
    }
}

/**
 * Marks `path`'s model as having received an edit no mounted `EditorPane` observed, and notifies
 * every {@link onModelEditedExternally} listener. The two are one call on purpose: an edit that sets
 * the in-memory flag but not the tab's own dirty flag is exactly the state that let a rename's
 * background half be closed (⌘W, "close saved tabs") with no confirmation and no dirty dot — see
 * `workspace-edit-applier.ts`'s call site.
 */
export const markModelDirtyExternally = (path: string) => {
    externallyDirtyPaths.add(path)
    for (const listener of externalEditListeners) listener(path)
}

/** Returns whether `path` was marked dirty and clears the mark — a one-shot flag consumed the first time a tab syncs against it. */
export const consumeExternallyDirtyModel = (path: string) => {
    const wasDirty = externallyDirtyPaths.has(path)
    externallyDirtyPaths.delete(path)
    return wasDirty
}

/**
 * Drops `path`'s mark without reporting it, for the one case where no tab is left to consume it:
 * `entities/layout/tab-path-change.ts`'s `releaseClosedFileTabPath` disposing the model of a path
 * that just closed everywhere.
 *
 * The `Set` is keyed by bare path and lives for the whole renderer session, so a mark left behind
 * outlives the model it described. Reopening that path builds a *fresh* model from disk, and the
 * first `EditorPane` to attach to it would read the stale mark through
 * {@link consumeExternallyDirtyModel} and conclude an unobserved edit is already sitting in a buffer
 * that has never been touched — suppressing the disk sync that fills it, and marking the tab dirty
 * against content identical to the file.
 */
export const clearExternallyDirtyMark = (path: string) => {
    externallyDirtyPaths.delete(path)
}
