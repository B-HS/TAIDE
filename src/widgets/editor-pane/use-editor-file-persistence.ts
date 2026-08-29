import type { Dispatch, RefObject, SetStateAction } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { MirrorEntry, OpenedFile, ProjectId, TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { HOT_EXIT_MIRROR_DEBOUNCE_MS } from '@shared/constants/mirror'
import { QUERY_KEY } from '@shared/constants/query-key'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { runOnSaveCleanup } from '@shared/lib/monaco/on-save-cleanup'
import { resolveOnSaveCleanupFlags } from '@shared/lib/editorconfig'
import { fileMirrorsQueryOptions, useSaveFile } from '@entities/file/file.query'
import { clearMirror, mirrorDirty } from '@entities/file/file.ipc'
import type { useSetTabDirty } from '@entities/layout/layout.query'
import { applyExternalContent } from '@entities/editor/model-registry'
import { publishFileSaveSettle, subscribeFileSaveSettle } from '@entities/editor/file-save-settle-registry'
import { registerMirrorFlush, unregisterMirrorFlush } from '@entities/editor/mirror-flush-registry'
import { readDraftSafely, shouldSettleDraftAfterDiskWrite } from '@widgets/editor-pane/editor-draft-sync'
import type { ConflictBannerVariant } from '@features/editor/conflict-banner'

const MARKDOWN_PREVIEW_DEBOUNCE_MS = 200
const FORMAT_DOCUMENT_ACTION_ID = 'editor.action.formatDocument'

/**
 * Reads this pane's current draft text on demand. `CodeEditor` hands one of these to `onChange`
 * instead of the text itself (see `CodeEditorProps.onChange`), so a keystroke costs a closure rather
 * than a full copy of the document; the debounced consumers below (hot-exit mirror, markdown
 * preview) call it when they actually fire, and the save path calls it when it actually saves.
 */
type DraftReader = () => string

const constantDraft =
    (content: string): DraftReader =>
    () =>
        content

type UseEditorFilePersistenceInput = {
    projectId: ProjectId
    path: string
    tabId: TabId
    file: OpenedFile | undefined
    autoSaveDelayMs: number | undefined
    formatOnSave: boolean | undefined
    trimTrailingWhitespaceOnSave: boolean | undefined
    insertFinalNewlineOnSave: boolean | undefined
    isMarkdown: boolean
    editor: monaco.editor.IStandaloneCodeEditor | null
    setSyncedContent: Dispatch<SetStateAction<string | null>>
    setTabDirty: ReturnType<typeof useSetTabDirty>['mutate']
    notifyLspSessionsOfSave: () => Promise<void>
    runCodeActionsOnSave: () => Promise<void>
    previewTimeoutRef: RefObject<ReturnType<typeof setTimeout> | undefined>
    setPreviewSource: (value: string | null) => void
    t: ReturnType<typeof useTranslation>['t']
}

/**
 * Owns `EditorPane`'s hot-exit mirror (the debounced draft snapshot IPC persists so an unsaved
 * buffer survives a crash/close) and the save pipeline (⌘S, auto-save, format-on-save, Code
 * Actions on Save, and the "changed on disk" / mirror-restored banner). The two are combined
 * because they share the same draft/epoch bookkeeping: every write path — a keystroke, an
 * explicit save, "view disk", a mirror restore — has to agree on which draft is current and
 * whether a given mirror write is still valid, so splitting them would mean splitting those refs
 * across files instead of concerns.
 */
export const useEditorFilePersistence = ({
    projectId,
    path,
    tabId,
    file,
    autoSaveDelayMs,
    formatOnSave,
    trimTrailingWhitespaceOnSave,
    insertFinalNewlineOnSave,
    isMarkdown,
    editor,
    setSyncedContent,
    setTabDirty,
    notifyLspSessionsOfSave,
    runCodeActionsOnSave,
    previewTimeoutRef,
    setPreviewSource,
    t,
}: UseEditorFilePersistenceInput) => {
    const draftRef = useRef<DraftReader | null>(null)
    /**
     * The live counterpart of the `dirty` state below, written in the same statement so it is true
     * the instant a dirty transition happens rather than one re-render later. `EditorPane`'s
     * disk-content sync runs from a microtask queued in the very commit a hot-exit mirror restore is
     * also queued in (see `editor-draft-sync.ts`'s `syncModelFromDisk`), so a guard reading the
     * render's `dirty` snapshot is stale by construction there and overwrites the restored buffer.
     */
    const dirtyRef = useRef(false)
    const pendingMirrorRef = useRef(false)
    const savingRef = useRef(false)
    const saveEpochRef = useRef(0)
    /**
     * Monotonic counter bumped once per `persistMirror` write attempt that actually reaches the
     * IPC call (not the writes that bail out on the pre-check). Lets a write's post-await epoch
     * mismatch tell "a newer write already started for this path" (skip reverting — that newer
     * write owns the outcome) apart from "nothing newer is coming" (safe to revert). Without this,
     * `persistMirror`'s revert branch unconditionally cleared the mirror on any epoch mismatch,
     * which could delete a *different*, already-committed write for the same path that simply
     * finished first (a save landing between two overlapping mirror writes — debounce vs. a
     * blur/hot-exit flush racing it).
     */
    const mirrorWriteSeqRef = useRef(0)
    const mirrorTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const autoSaveTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    /**
     * Mirrors the `path` prop — kept fresh by the `[path]` effect below rather than read directly,
     * so a stale closure can tell "still the same path" apart from "armed for a path this instance
     * has since moved on from". `EditorPane` has no `key` (see the mirror-cache doc comment
     * further below), so switching tabs in the same pane reuses this instance and its in-flight
     * timers rather than remounting them away; `handleChange`'s auto-save and markdown-preview
     * timers check this at fire time against the `scheduledPath` they captured when armed.
     */
    const pathRef = useRef(path)

    const [dirty, setDirtyState] = useState(false)
    const [restoreNotice, setRestoreNotice] = useState<Exclude<ConflictBannerVariant, 'changedOnDisk'> | 'none'>('none')

    const queryClient = useQueryClient()
    const { data: mirrors } = useQuery(fileMirrorsQueryOptions(projectId))
    const { mutate: saveFile } = useSaveFile(projectId)

    /**
     * The only way this hook (and `EditorPane`, through the returned `setDirty`) changes dirtiness —
     * state and {@link dirtyRef} always move together, so no caller can leave the live signal behind.
     * `EditorPane` calls it from its render-phase path-switch reset, which is safe because the ref
     * write only ever mirrors the `setState` made in the same statement: React re-renders that
     * component immediately with the new state, so ref and state can never disagree afterwards.
     */
    const setDirty = (next: boolean) => {
        dirtyRef.current = next
        setDirtyState(next)
    }

    const readDraft = () => readDraftSafely(draftRef.current)

    const isDraftDirty = () => dirtyRef.current

    /**
     * Writes to the hot-exit mirror and keeps the `FILE.MIRRORS` query cache in lockstep, instead
     * of leaving it at its `staleTime: Infinity` project-activation snapshot until the next
     * save/view-disk/prune. Without this, revisiting this tab after a same-pane detour to another
     * tab (`EditorPane` has no `key`, so a path switch reuses this instance) would restore from a
     * stale cached entry — either missing entirely (this tab's edits since activation never landed
     * in the cache) or, worse, an *older* mirror than what's already on screen, silently rolling
     * back newer edits. The cache write reuses whatever `disk_modified_ms` baseline `mirrorDirty`
     * (the IPC call) reports back — the backend derives that live from the file's actual on-disk
     * mtime at the moment it writes the mirror (`file/service.rs`'s `mirror_dirty`), not from
     * anything this component tracks itself. That matters specifically because this function's
     * *own* closure can go stale: it can run from a previous render's `flush` (the mirror-flush
     * effect's cleanup, invoked when `file?.modifiedMs` changes — see that effect's own doc
     * comment) after a save has already landed a newer `modifiedMs`. Reading `file?.modifiedMs`
     * directly here (the earlier design) would silently use whichever render's `file` that
     * particular closure happened to capture — stale relative to the disk by the time the write
     * actually reaches the backend, and wrongly baselined as a result (false "restored" conflict
     * banner on next launch even though nothing external touched the file). Sourcing the baseline
     * from the backend's own live read instead makes which closure ran irrelevant.
     *
     * `epoch` is the value of `saveEpochRef` at the moment this write was *scheduled* (armed by the
     * debounce timer or a flush), not when it runs — `handleSave`'s `onSuccess`/`handleViewDisk`
     * bump `saveEpochRef` the instant disk is known to match, and Rust's own `file_save` already
     * clears the mirror in that same request. A write scheduled before that bump can otherwise land
     * *after* the clear (the backend's mutation lock is FIFO but no longer orders these two commands
     * relative to each other — see `file_mirror_dirty`'s doc comment), resurrecting a mirror for a
     * file that's already clean and triggering a false "restored" conflict banner on next launch.
     * Checked twice — before the IPC call (skip entirely if already stale) and after it resolves (a
     * save can complete *during* the round trip).
     *
     * The post-await mismatch does *not* always mean "revert" though — two further cases are
     * distinguished before falling back to that:
     *  1. A *newer* write has already started for this path (`mirrorWriteSeqRef` moved past the
     *     sequence number this call claimed) — that write owns the outcome; reverting here would
     *     risk clobbering whatever it already committed (or is about to), so this call just backs
     *     off instead.
     *  2. `content` is still exactly the live draft and a mirror is still owed for it (`draftRef`
     *     unchanged, `pendingMirrorRef` still set) — the epoch bump came from a save that finished
     *     concurrently with (not before) this content existing, so it's not stale, only mis-baselined
     *     against a now-outdated epoch/disk snapshot. Retried once against the current epoch instead
     *     of being deleted — otherwise a hot-exit flush racing a same-tick save's `onSuccess` could
     *     write the very last unsaved edit and then immediately erase it right as the window closes.
     * Only when neither holds is the write genuinely superseded, and the mirror reverted via
     * `clearMirror` (fire-and-forget, matching this function's other IPC side effects) with the
     * cache left alone so the stale entry never becomes visible.
     *
     * Returns whether the write actually took effect, so callers know whether it's safe to treat the
     * outstanding edit as flushed (see `handleChange`'s and the mirror-flush effect's use of this).
     *
     * Like {@link settleDraftToDiskContent}'s and `entities/layout/tab-path-change.ts`'s patches, the
     * cache updater hands back `undefined` — TanStack Query's "leave the query alone" — when the
     * project's mirror list has never been fetched: seeding a one-entry list into that
     * `staleTime: Infinity` query would state that this is the project's *only* mirror. The backend
     * already holds this write, so the list's own first fetch reports it anyway.
     */
    const persistMirror = async (content: string, epoch: number): Promise<boolean> => {
        if (epoch !== saveEpochRef.current) return false
        const writeSeq = ++mirrorWriteSeqRef.current
        const diskModifiedMs = await mirrorDirty({ projectId, path, content })
        if (epoch === saveEpochRef.current) {
            queryClient.setQueryData(QUERY_KEY.FILE.MIRRORS(projectId), (previous?: MirrorEntry[]) =>
                previous
                    ? [...previous.filter((entry) => entry.path !== path), { path, content, savedAtMs: Date.now(), diskModifiedMs, conflict: false }]
                    : undefined,
            )
            return true
        }
        if (writeSeq !== mirrorWriteSeqRef.current) return false
        if (readDraft() === content && pendingMirrorRef.current) return persistMirror(content, saveEpochRef.current)
        void clearMirror({ projectId, path }).catch(() => undefined)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
        return false
    }

    /**
     * `readContent` is a lazy reader for the model's text ({@link DraftReader}), not the text — the
     * keystroke path never materializes the document. Both debounced consumers below call it at fire
     * time instead, which is both cheaper (one string per debounce window rather than per character)
     * and fresher: the reader is bound to the model that changed, so a path switch cannot redirect it
     * (`scheduledPath` still guards the *effects* of those timers, which must not apply to another
     * tab's state).
     */
    const handleChange = (readContent: DraftReader) => {
        draftRef.current = readContent
        if (!dirtyRef.current) {
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
        }

        pendingMirrorRef.current = true
        clearTimeout(mirrorTimeoutRef.current)
        /**
         * Captured now, not read inside the timeout — `saveEpochRef` can bump while this timer is
         * still counting down (a save started before this keystroke can finish after it). Passed
         * straight through to `persistMirror`, which compares it against the *current* epoch both
         * before writing and after the IPC round trip; see that function's doc comment.
         */
        const scheduledEpoch = saveEpochRef.current
        mirrorTimeoutRef.current = setTimeout(() => {
            const content = readDraftSafely(readContent)
            if (content === null) return
            void persistMirror(content, scheduledEpoch)
                .then((committed) => {
                    if (committed) pendingMirrorRef.current = false
                })
                .catch(() => undefined)
        }, HOT_EXIT_MIRROR_DEBOUNCE_MS)

        /**
         * Guarded by `savingRef` so an edit `handleSave` itself causes mid-save (formatOnSave's
         * `formatDocument` action, or the Code Actions on Save applier below, both push edits
         * through the same `onDidChangeContent` → `onChange` → here path) doesn't re-arm a second
         * auto-save on top of the one already in flight.
         *
         * `scheduledPath` is captured here (this render's `path`, guaranteed current) and checked
         * against `pathRef.current` when the timer fires, not before. Without it, switching this
         * pane to a different tab before the delay elapses would still fire — this instance has no
         * `key`, so the timer's own closure keeps pointing at the old path and old `draftRef`
         * snapshot — and `handleSave` would write the *new* tab's since-typed content to disk under
         * the *old* tab's path. Applied to the markdown-preview timer below for the same reason,
         * though there the consequence is only a wrong preview render rather than a wrong write.
         */
        const scheduledPath = path
        const resolvedAutoSaveDelayMs = autoSaveDelayMs ?? 0
        clearTimeout(autoSaveTimeoutRef.current)
        if (resolvedAutoSaveDelayMs > 0 && !savingRef.current) {
            autoSaveTimeoutRef.current = setTimeout(() => {
                if (pathRef.current === scheduledPath) void handleSave('auto')
            }, resolvedAutoSaveDelayMs)
        }

        if (!isMarkdown) return
        clearTimeout(previewTimeoutRef.current)
        previewTimeoutRef.current = setTimeout(() => {
            const preview = readDraftSafely(readContent)
            if (preview !== null && pathRef.current === scheduledPath) setPreviewSource(preview)
        }, MARKDOWN_PREVIEW_DEBOUNCE_MS)
    }

    /**
     * Adopts an edit that landed on this path's model while no mounted `EditorPane` was watching it
     * (an LSP `WorkspaceEdit` on a background tab — `shared/lib/lsp/model-dirty-tracker.ts`) as this
     * pane's own unsaved draft, instead of `EditorPane`'s sync overwriting it with the disk content.
     * Lives here rather than in `EditorPane` so this hook's draft/dirty bookkeeping stays behind one
     * door, the same reason {@link settleAfterDiskWrite} exists for the git-gutter hook.
     */
    const adoptUnobservedModelEdit = (readContent: DraftReader) => {
        draftRef.current = readContent
        setDirty(true)
        setTabDirty({ tabId, dirty: true })
    }

    /**
     * `reason` distinguishes an explicit save (⌘S, the default — `CodeEditor`'s `onSave` calls
     * this with no arguments) from an auto-save re-trigger (`handleChange` passes `'auto'`
     * explicitly). Code Actions on Save only runs for explicit saves — auto-save firing mid-typing
     * is exactly the "don't surprise-mutate the buffer" case the boolean settings' description
     * warns against, and untitled tabs never reach this component's save path at all.
     *
     * A `readOnly` file is never written back. Monaco already refuses the keystrokes, so normally
     * no draft exists to save — but a hot-exit mirror restore (`applyMirrorRestore`) installs a
     * draft without going through the editor, and for a file forced read-only by
     * `encodingLossy` (bytes that are not valid UTF-8, decoded with U+FFFD replacements —
     * `OpenedFile.encodingLossy`) saving that draft would burn those replacements into the file
     * permanently. The tier-based read-only files (`large`/`readOnly`) get the same treatment for
     * free, which matches what their banner has always claimed.
     *
     * An explicit ⌘S says so out loud instead of doing nothing: the only way this branch is reached
     * is a tab that *looks* dirty (a restored mirror, an adopted background edit), so a silent
     * no-op reads as "saving is broken". Auto-save stays silent — it fires on a timer the user did
     * not press.
     */
    const handleSave = async (reason: 'explicit' | 'auto' = 'explicit') => {
        if (file?.readOnly) {
            if (reason === 'explicit') toast.error(t('editor.readOnlySaveBlocked'))
            return
        }
        if (readDraft() === null) return

        savingRef.current = true
        clearTimeout(autoSaveTimeoutRef.current)

        if (reason === 'explicit') await runCodeActionsOnSave().catch(() => undefined)

        if (formatOnSave) {
            const formatAction = editor?.getAction(FORMAT_DOCUMENT_ACTION_ID)
            if (formatAction) await formatAction.run().catch(() => undefined)
        }

        /**
         * Last writer before the draft is read, so the formatter's own trailing whitespace is
         * cleaned too — and so every save trigger that reaches this function (⌘S, auto-save,
         * format-on-save) gets the same treatment from one place rather than each arming its own.
         * The cleanup edits go through the model, so `handleChange` has already refreshed
         * `draftRef` by the time `readDraft()` below runs; `savingRef` (set above) keeps that
         * re-entry from arming a second auto-save, exactly as it does for the format step.
         *
         * Which of the two cleanups run is this file's decision, not only the global settings':
         * `resolveOnSaveCleanupFlags` lets the `trim_trailing_whitespace`/`insert_final_newline` its
         * `.editorconfig` chain resolved to (d-53 U3) win over the matching setting, including an
         * explicit `false` against a global `true`. The properties arrive on `OpenedFile`, so they
         * are whatever was in force when this tab was opened.
         */
        await runOnSaveCleanup({
            editor,
            ...resolveOnSaveCleanupFlags({ trimTrailingWhitespaceOnSave, insertFinalNewlineOnSave }, file?.editorConfig),
            isAutoSave: reason === 'auto',
        })

        const finalContent = readDraft()
        if (finalContent === null) {
            savingRef.current = false
            return
        }

        saveFile(
            { path, content: finalContent },
            {
                /**
                 * `saveEpochRef` bumps unconditionally — Rust's `file_save` already cleared the
                 * mirror server-side for `finalContent` regardless of what's typed since, so any
                 * mirror write `persistMirror` scheduled before this point must not be trusted to
                 * still reflect reality (see `persistMirror`'s doc comment). *Not* bumped on
                 * `onError` below — a failed save changes nothing on disk, so a write already in
                 * flight for the pre-save content is still exactly the recovery data hot exit needs.
                 *
                 * The rest of this success handling only fires when the live draft still equals
                 * `finalContent` — i.e. nothing was typed during the save round trip. If it doesn't
                 * match, the user kept typing while the save/format/code-actions were in flight; that
                 * content was never sent to disk and must stay `dirty` with its mirror timer left
                 * armed (it will find `pendingMirrorRef` still true and, thanks to the epoch bump
                 * above, either fire successfully with a fresh schedule or fall through to the
                 * mirror-flush effect — either way the still-unsaved edit reaches the mirror, never
                 * gets silently marked clean, and is never clobbered by the `FILE.CONTENT` refetch
                 * this mutation's `onSuccess` triggers.
                 *
                 * `setSyncedContent(finalContent)` below restores this hook's `syncedContent`
                 * invariant ("last known disk content") the instant the save is known to have landed,
                 * instead of leaving it holding the pre-save content until a separate `FILE.CONTENT`
                 * refetch (armed by `file.modifiedMs` changing) happens to land. `editor-pane.tsx`'s
                 * `[editor, syncedContent, dirty, path]` effect re-applies `syncedContent` via
                 * `applyExternalContent` on every dirty→false transition it observes; without this
                 * line, a refetch that loses the race (a slow/remote round trip) leaves that effect
                 * firing against the still-stale pre-save `syncedContent`, clobbering the
                 * just-typed-and-saved buffer back to its pre-save contents and re-marking it dirty
                 * with no path back to clean (docs/acknowledge/2026-08-27-d43-save-stale-sync-clobber-
                 * contract.md §0). Restoring the invariant here makes that effect's re-apply a
                 * same-content no-op instead, closing the race window entirely rather than trying to
                 * win it.
                 */
                onSuccess: () => {
                    savingRef.current = false
                    saveEpochRef.current += 1
                    if (readDraft() === finalContent) settleDraftToDiskContent(finalContent)
                    void notifyLspSessionsOfSave()
                },
                onError: (saveError) => {
                    savingRef.current = false
                    toast.error(describeIpcError(saveError))
                },
            },
        )
    }

    /**
     * The single "this pane's buffer now matches what is on disk" transition: bump the save epoch so
     * any mirror write scheduled against the old disk state is refused (`persistMirror`), disarm the
     * pending mirror debounce, restore the `syncedContent` invariant ("last known disk content") in
     * the same tick as the dirty reset, and drop this path's now-obsolete `FILE.MIRRORS` cache entry.
     *
     * `content` is the exact text that reached disk — callers must pass what they actually wrote.
     * Forwarding it into `setSyncedContent` is what keeps `editor-pane.tsx`'s dirty→false sync a
     * same-content no-op instead of a race against the confirming `FILE.CONTENT` refetch, which if
     * lost would clobber the just-written buffer back to its pre-write contents and re-mark it dirty
     * with no path back to clean (docs/acknowledge/2026-08-27-d43-save-stale-sync-clobber-contract.md
     * §0).
     *
     * The `FILE.MIRRORS` patch is what stops the mirror from coming back to life: every disk write
     * that reaches this function also discards the backend mirror for `path`, but the cached mirror
     * list is `staleTime: Infinity` (`fileMirrorsQueryOptions`), so a pane that later re-mounts on
     * this path would find the pre-write entry still sitting in the cache and "restore" it over a
     * file that is already clean. Patched rather than invalidated so the fix holds even for writes
     * whose own mutation has no project scope to invalidate (`ide-sync-provider`'s save). The updater
     * returns `undefined` — which TanStack Query treats as "leave the query alone" — whenever there
     * is nothing to remove: seeding `[]` into an unfetched `staleTime: Infinity` query would
     * permanently convince every pane in the project that no mirrors exist, and handing back a fresh
     * array on every save would re-render every mirror observer in the project for no change.
     */
    const settleDraftToDiskContent = (content: string) => {
        saveEpochRef.current += 1
        clearTimeout(mirrorTimeoutRef.current)
        pendingMirrorRef.current = false
        setSyncedContent(content)
        /**
         * The tab-dirty IPC is skipped for a pane that is already clean — every save on a path now
         * reaches every pane showing it, and a `layout_set_tab_dirty` per clean pane per save would
         * invalidate (and refetch) the whole project layout for a flag that never changed.
         */
        if (dirtyRef.current) {
            setDirty(false)
            setTabDirty({ tabId, dirty: false })
        }
        setRestoreNotice('none')
        queryClient.setQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(projectId), (previous) =>
            previous?.some((entry) => entry.path === path) ? previous.filter((entry) => entry.path !== path) : undefined,
        )
    }

    /**
     * The settle a disk write performed *outside* this hook reports back — currently
     * `useEditorGitGutterAndConflicts`'s merge-conflict resolution, which already holds the exact
     * text it both `executeEdits`-applied and sent to `git_resolve_conflict`. Handed out instead of
     * `saveEpochRef`/`mirrorTimeoutRef`/`pendingMirrorRef`/`setSyncedContent`/`setDirty`/`setTabDirty`
     * themselves so what a hook outside this file can do to this hook's bookkeeping stays "settle",
     * not "mutate the refs directly".
     *
     * Also announces the write on `file-save-settle-registry`, because the write landed on a *path*,
     * not on this pane: a split view can have a second `EditorPane` on the same file (sharing one
     * monaco model), and without the announcement that pane keeps its own stale `dirty` /
     * `syncedContent` / armed mirror write.
     */
    const settleAfterDiskWrite = (content: string) => {
        settleDraftToDiskContent(content)
        publishFileSaveSettle(path, content)
    }

    const handleViewDisk = () => {
        if (!file) return

        const diskContent = file.content
        draftRef.current = constantDraft(diskContent)
        settleDraftToDiskContent(diskContent)
        setPreviewSource(null)
        void clearMirror({ projectId, path }).catch(() => undefined)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
    }

    const handleKeepMine = () => {
        if (file) setSyncedContent(file.content)
        setRestoreNotice('none')
    }

    /**
     * `useEffectEvent` so the restore below can read the always-current `editor`/`path`/`tabId`
     * without being a reactive dependency of the effect that calls it — the effect's own guard
     * (`draftRef.current !== null`) is what decides whether a restore is due, not this event's deps.
     */
    const applyMirrorRestore = useEffectEvent((mirror: MirrorEntry) => {
        if (!editor) return
        applyExternalContent(path, mirror.content, editor)
        draftRef.current = constantDraft(mirror.content)
        setDirty(true)
        setTabDirty({ tabId, dirty: true })
        setRestoreNotice(mirror.conflict ? 'mirrorRestoredConflict' : 'mirrorRestored')
    })

    useEffect(() => {
        draftRef.current = null
        pathRef.current = path
    }, [path])

    /**
     * Restores unsaved edits from the hot-exit mirror the first time the monaco editor is mounted
     * and nothing has been typed yet for this `path` (a fresh mount or a path switch, never
     * mid-edit — the reset effect above always runs first for a path switch within the same commit).
     * `mirror.conflict` (computed in Rust from `disk_modified_ms` vs the mirror's baseline) decides
     * whether this surfaces as a plain restore notice or a conflict requiring the user to choose.
     */
    useEffect(() => {
        if (!editor || draftRef.current !== null) return
        const mirror = (mirrors ?? []).find((entry) => entry.path === path)
        if (mirror) queueMicrotask(() => applyMirrorRestore(mirror))
    }, [editor, path, mirrors])

    /**
     * Registers a flush callback the hot-exit `CloseRequested` handler (and window blur / a path
     * switch that unmounts this content) can invoke to push the last debounced edit to the mirror
     * immediately, instead of losing up to `HOT_EXIT_MIRROR_DEBOUNCE_MS` of unmirrored edits.
     * `pendingMirrorRef` (not `dirty`) gates the write so a save/view-disk that clears the mirror
     * right before this effect's `file.modifiedMs` dependency changes can't race a stale flush back
     * into existence — reinforced by `persistMirror`'s own epoch check (`saveEpochRef.current`, read
     * fresh at flush time rather than captured ahead like the debounce timer's `scheduledEpoch`,
     * since a flush always fires in direct response to a real event, never on a timer of its own).
     * Only clears `pendingMirrorRef` when `persistMirror` reports the write actually committed — if a
     * save raced this and won, the edit stays flagged pending so the *next* flush (now past the save,
     * with a current epoch) retries it against the live draft instead of the attempt being silently
     * dropped.
     */
    useEffect(() => {
        const flush = async () => {
            clearTimeout(mirrorTimeoutRef.current)
            if (!pendingMirrorRef.current) return
            const draft = readDraft()
            if (draft === null) return
            const committed = await persistMirror(draft, saveEpochRef.current).catch(() => false)
            if (committed) pendingMirrorRef.current = false
        }

        registerMirrorFlush(tabId, flush)
        window.addEventListener('blur', flush)
        return () => {
            window.removeEventListener('blur', flush)
            void flush()
            unregisterMirrorFlush(tabId)
        }
    }, [projectId, path, tabId, file?.modifiedMs])

    /**
     * Settles this pane whenever *anything* writes this path to disk — this pane's own ⌘S/auto-save,
     * the other half of a split view showing the same file, a Claude Code `ide:save-requested` save
     * (`app/providers/ide-sync-provider.tsx`), or a merge-conflict resolution. Save success used to
     * settle only the `EditorPane` instance that issued it, so every other pane on the path kept its
     * own `dirty`, kept the pre-save text as its "last known disk content" (which reads as a spurious
     * "changed on disk" conflict the moment the refetch lands), and kept an armed mirror write that
     * resurrected a hot-exit mirror for an already-clean file.
     *
     * Guarded by {@link shouldSettleDraftAfterDiskWrite}: a pane typed into while the write was in
     * flight holds text that never reached disk and must stay dirty rather than be marked clean onto
     * content it does not have.
     */
    useEffect(
        () =>
            subscribeFileSaveSettle(path, (content) => {
                if (!shouldSettleDraftAfterDiskWrite(readDraft(), content)) return
                settleDraftToDiskContent(content)
            }),
        [projectId, path, tabId],
    )

    useEffect(() => () => clearTimeout(autoSaveTimeoutRef.current), [])

    return {
        dirty,
        isDraftDirty,
        setDirty,
        adoptUnobservedModelEdit,
        restoreNotice,
        setRestoreNotice,
        handleChange,
        handleSave,
        handleViewDisk,
        handleKeepMine,
        settleAfterDiskWrite,
    }
}
