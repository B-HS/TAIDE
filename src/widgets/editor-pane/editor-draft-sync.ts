type SyncModelFromDiskDeps = {
    isDraftDirty: () => boolean
    hasUnobservedModelEdit: () => boolean
    adoptUnobservedModelEdit: () => void
    applyDiskContent: () => void
}

/**
 * Runs `EditorPane`'s "make this pane's monaco model match the last known disk content" step, with
 * every input read at the moment it actually executes rather than captured when it was scheduled.
 *
 * That distinction is the whole point of this function. The sync is queued to a microtask by an
 * effect, and the hot-exit mirror restore (`use-editor-file-persistence.ts`) is queued to a
 * microtask by an effect in the SAME commit — the restore first, since its hook is called first.
 * The restore turns the pane dirty (its draft becomes the recovered unsaved buffer), but that
 * `setDirty` cannot re-render before the already-queued sync microtask runs, so a sync that reads
 * the render's `dirty` snapshot sees `false`, overwrites the just-restored buffer with the disk
 * text, and — because that overwrite is itself a model change — lets the mirror debounce persist
 * the disk text over the recovery data a moment later, destroying it. Reading `isDraftDirty()`
 * live instead makes the restore's synchronous dirty transition visible to this guard, which is
 * what actually orders the two operations rather than relying on a snapshot that is stale by
 * construction.
 *
 * `hasUnobservedModelEdit` is only consulted once the pane is known to be clean, preserving its
 * one-shot consume semantics (`shared/lib/lsp/model-dirty-tracker.ts`): a dirty pane already has
 * the edit in its own draft and must not burn the flag on its way past.
 */
export const syncModelFromDisk = ({ isDraftDirty, hasUnobservedModelEdit, adoptUnobservedModelEdit, applyDiskContent }: SyncModelFromDiskDeps) => {
    if (isDraftDirty()) return

    if (hasUnobservedModelEdit()) {
        adoptUnobservedModelEdit()
        return
    }

    applyDiskContent()
}

/**
 * Whether a pane may mark itself clean in response to a disk write of `writtenContent` for the path
 * it is showing — used both by the pane that issued the write and by every other pane settled
 * through `entities/editor/file-save-settle-registry.ts`.
 *
 * `draft` is the pane's live draft (`null` when nothing has been typed or restored for this path
 * yet, e.g. a second pane on the same file that has never been edited through). A draft that no
 * longer equals what reached disk means the buffer was typed into while the write was in flight;
 * that text was never sent to disk, so the pane has to stay dirty with its mirror timer armed
 * rather than being settled onto content it does not hold.
 */
export const shouldSettleDraftAfterDiskWrite = (draft: string | null, writtenContent: string) => draft === null || draft === writtenContent

/**
 * Reads a pane's draft through the lazy reader `CodeEditor` hands out (`() => model.getValue()` —
 * `use-editor-file-persistence.ts`'s `DraftReader`), reporting "no draft" instead of throwing when
 * the model that reader is bound to has already been disposed.
 *
 * A reader outliving its model is a normal, designed sequence, not a leak: `useCloseTab` disposes
 * the closed path's model (`entities/layout/tab-path-change.ts`'s `releaseClosedFileTabPath`) and a
 * rename disposes the old one (`retargetModel`), both *synchronously*, before React unmounts or
 * re-renders the pane whose refs still hold that reader. The pane's own teardown then runs — the
 * mirror-flush effect's cleanup flushes, the blur listener may fire — and monaco answers
 * `getValue()` on a disposed model with `BugIndicatingError('Model is disposed!')`, which inside an
 * `async` flush becomes an unhandled rejection that `shared/lib/error-log-forwarding.ts` records as
 * an app error on every close of a file that was ever typed into. There is nothing to persist at
 * that point (the close path already cleared the mirror, the rename path already migrated it), so
 * the honest answer is `null` — the same value every other "this pane has no draft" case yields.
 */
export const readDraftSafely = (reader: (() => string) | null) => {
    if (!reader) return null
    try {
        return reader()
    } catch {
        return null
    }
}

type AdoptLiveModelEditInput = {
    /** `CodeEditor`'s `onModelAttach` report for the attach this pane is reconciling: whether the buffer it was handed already existed. */
    hadLiveModelOnAttach: boolean
    /** The attached model's current text, `null` when there is no model to read (the editor is not mounted yet). */
    modelContent: string | null
    /** The last known on-disk text for the same path, `null` while the file query has not resolved. */
    diskContent: string | null
}

/**
 * Whether a pane mounting onto a path must take the monaco model's current text as its own unsaved
 * draft instead of treating the buffer as reconcilable against disk.
 *
 * A model is shared by every pane showing the same path (`entities/editor/model-registry.ts` keys it
 * by path, and `open_tab`'s dedupe is per-leaf, so a split view legitimately puts two panes on one
 * buffer). A pane that mounts onto an ALREADY-LIVE model therefore inherits a buffer whose edits its
 * own `onDidChangeModelContent` never saw: it starts `dirty: false` with a draft of `null` while the
 * text on screen is somebody else's unsaved work. Both of the reconciliations that then run would
 * destroy it — `editor-pane.tsx`'s disk sync `setValue`s the model back to the file's on-disk text
 * (audit §2-7), and `use-editor-file-persistence.ts`'s hot-exit restore rolls it back to a mirror
 * snapshot up to `HOT_EXIT_MIRROR_DEBOUNCE_MS` old while claiming a crash recovery in the banner
 * (audit §2-8) — and neither is observable, because `applyExternalContent` masks its own change
 * event from every editor on the model.
 *
 * Both halves are required. `hadLiveModelOnAttach` is what separates "this buffer is somebody's live
 * draft" from "the disk text moved while this pane was clean": only the attach can see the
 * difference, since after `getOrCreateModel` the model always exists, and a pane that has been
 * mounted all along has been observing every change itself. Without it, an external write picked up
 * by the file watcher — where the model legitimately still holds the OLD disk text and must be
 * updated to the new one — would be misread as an unobserved edit and freeze the stale text as a
 * bogus unsaved draft. The content comparison is what keeps the ordinary split-view case (a second
 * pane on a file nobody has edited) on the normal path.
 */
export const shouldAdoptLiveModelEdit = ({ hadLiveModelOnAttach, modelContent, diskContent }: AdoptLiveModelEditInput) =>
    hadLiveModelOnAttach && modelContent !== null && diskContent !== null && modelContent !== diskContent

type ChangedOnDiskConflictInput = {
    isDirty: boolean
    syncedContent: string | null
    diskContent: string | null
}

/**
 * Whether the pane must show the "changed on disk" banner: it holds unsaved edits AND what it knows
 * as the on-disk text no longer matches what the file query reports. Both halves matter — a pane
 * whose `syncedContent` was never settled after someone else's save reports a conflict that does not
 * exist, since the only thing that changed on disk is the content it is already showing.
 */
export const hasChangedOnDiskConflict = ({ isDirty, syncedContent, diskContent }: ChangedOnDiskConflictInput) =>
    isDirty && syncedContent !== null && diskContent !== null && diskContent !== syncedContent
