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

/** Marks `path`'s model as having received an edit no mounted `EditorPane` observed. */
export const markModelDirtyExternally = (path: string) => {
    externallyDirtyPaths.add(path)
}

/** Returns whether `path` was marked dirty and clears the mark — a one-shot flag consumed the first time a tab syncs against it. */
export const consumeExternallyDirtyModel = (path: string) => {
    const wasDirty = externallyDirtyPaths.has(path)
    externallyDirtyPaths.delete(path)
    return wasDirty
}
