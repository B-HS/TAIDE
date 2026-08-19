import type { Dispatch, RefObject, SetStateAction } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { MirrorEntry, OpenedFile, ProjectId, TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { HOT_EXIT_MIRROR_DEBOUNCE_MS } from '@shared/constants/mirror'
import { QUERY_KEY } from '@shared/constants/query-key'
import { fileMirrorsQueryOptions, useSaveFile } from '@entities/file/file.query'
import { clearMirror, mirrorDirty } from '@entities/file/file.ipc'
import type { useSetTabDirty } from '@entities/layout/layout.query'
import { applyExternalContent } from '@entities/editor/model-registry'
import { registerMirrorFlush, unregisterMirrorFlush } from '@entities/editor/mirror-flush-registry'
import type { ConflictBannerVariant } from '@features/editor/conflict-banner'

const MARKDOWN_PREVIEW_DEBOUNCE_MS = 200
const FORMAT_DOCUMENT_ACTION_ID = 'editor.action.formatDocument'

type UseEditorFilePersistenceInput = {
    projectId: ProjectId
    path: string
    tabId: TabId
    file: OpenedFile | undefined
    autoSaveDelayMs: number | undefined
    formatOnSave: boolean | undefined
    isMarkdown: boolean
    editor: monaco.editor.IStandaloneCodeEditor | null
    setSyncedContent: Dispatch<SetStateAction<string | null>>
    setTabDirty: ReturnType<typeof useSetTabDirty>['mutate']
    notifyLspSessionsOfSave: () => Promise<void>
    runCodeActionsOnSave: () => Promise<void>
    previewTimeoutRef: RefObject<ReturnType<typeof setTimeout> | undefined>
    setPreviewSource: (value: string | null) => void
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
    isMarkdown,
    editor,
    setSyncedContent,
    setTabDirty,
    notifyLspSessionsOfSave,
    runCodeActionsOnSave,
    previewTimeoutRef,
    setPreviewSource,
}: UseEditorFilePersistenceInput) => {
    const draftRef = useRef<string | null>(null)
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

    const [dirty, setDirty] = useState(false)
    const [restoreNotice, setRestoreNotice] = useState<Exclude<ConflictBannerVariant, 'changedOnDisk'> | 'none'>('none')

    const queryClient = useQueryClient()
    const { data: mirrors } = useQuery(fileMirrorsQueryOptions(projectId))
    const { mutate: saveFile } = useSaveFile(projectId)

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
     */
    const persistMirror = async (content: string, epoch: number): Promise<boolean> => {
        if (epoch !== saveEpochRef.current) return false
        const writeSeq = ++mirrorWriteSeqRef.current
        const diskModifiedMs = await mirrorDirty({ projectId, path, content })
        if (epoch === saveEpochRef.current) {
            queryClient.setQueryData(QUERY_KEY.FILE.MIRRORS(projectId), (previous?: MirrorEntry[]) => [
                ...(previous ?? []).filter((entry) => entry.path !== path),
                { path, content, savedAtMs: Date.now(), diskModifiedMs, conflict: false },
            ])
            return true
        }
        if (writeSeq !== mirrorWriteSeqRef.current) return false
        if (draftRef.current === content && pendingMirrorRef.current) return persistMirror(content, saveEpochRef.current)
        void clearMirror({ projectId, path }).catch(() => undefined)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
        return false
    }

    const handleChange = (value: string) => {
        draftRef.current = value
        if (!dirty) {
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
            void persistMirror(value, scheduledEpoch)
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
            if (pathRef.current === scheduledPath) setPreviewSource(value)
        }, MARKDOWN_PREVIEW_DEBOUNCE_MS)
    }

    /**
     * `reason` distinguishes an explicit save (⌘S, the default — `CodeEditor`'s `onSave` calls
     * this with no arguments) from an auto-save re-trigger (`handleChange` passes `'auto'`
     * explicitly). Code Actions on Save only runs for explicit saves — auto-save firing mid-typing
     * is exactly the "don't surprise-mutate the buffer" case the boolean settings' description
     * warns against, and untitled tabs never reach this component's save path at all.
     */
    const handleSave = async (reason: 'explicit' | 'auto' = 'explicit') => {
        const content = draftRef.current
        if (content === null) return

        savingRef.current = true
        clearTimeout(autoSaveTimeoutRef.current)

        if (reason === 'explicit') await runCodeActionsOnSave().catch(() => undefined)

        if (formatOnSave) {
            const formatAction = editor?.getAction(FORMAT_DOCUMENT_ACTION_ID)
            if (formatAction) await formatAction.run().catch(() => undefined)
        }

        const finalContent = draftRef.current
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
                 * The rest of this success handling only fires when `draftRef.current` still equals
                 * `finalContent` — i.e. nothing was typed during the save round trip. If it doesn't
                 * match, the user kept typing while the save/format/code-actions were in flight; that
                 * content was never sent to disk and must stay `dirty` with its mirror timer left
                 * armed (it will find `pendingMirrorRef` still true and, thanks to the epoch bump
                 * above, either fire successfully with a fresh schedule or fall through to the
                 * mirror-flush effect — either way the still-unsaved edit reaches the mirror, never
                 * gets silently marked clean, and is never clobbered by the `FILE.CONTENT` refetch
                 * this mutation's `onSuccess` triggers.
                 */
                onSuccess: () => {
                    savingRef.current = false
                    saveEpochRef.current += 1
                    if (draftRef.current === finalContent) {
                        clearTimeout(mirrorTimeoutRef.current)
                        pendingMirrorRef.current = false
                        setDirty(false)
                        setTabDirty({ tabId, dirty: false })
                        setRestoreNotice('none')
                    }
                    void notifyLspSessionsOfSave()
                },
                onError: (saveError) => {
                    savingRef.current = false
                    toast.error(saveError.message)
                },
            },
        )
    }

    /**
     * The save-epoch bump + mirror-timer/pending-flag clear + dirty reset that every "the buffer
     * now matches what's on disk" success path performs — `handleSave`'s own success handler (below)
     * and `handleViewDisk` (right below this) both inline the same sequence because each has its own
     * epoch-bump timing quirk (`handleSave` only bumps unconditionally but only settles the rest
     * inside its `draftRef.current === finalContent` guard; `handleViewDisk` does both unconditionally
     * but interleaves a few of its own statements in between) that folding into one shared call here
     * would either duplicate or subtly change. `useEditorGitGutterAndConflicts`'s own disk-write
     * success path (resolving a merge conflict) has no such quirk — it's the same five statements,
     * unconditionally — so it gets this function instead of `saveEpochRef`/`mirrorTimeoutRef`/
     * `pendingMirrorRef`/`setDirty`/`setTabDirty` themselves, narrowing what a hook outside this file
     * can do to this hook's internal bookkeeping to "settle", not "mutate the refs directly".
     */
    const settleAfterDiskWrite = () => {
        saveEpochRef.current += 1
        clearTimeout(mirrorTimeoutRef.current)
        pendingMirrorRef.current = false
        setDirty(false)
        setTabDirty({ tabId, dirty: false })
    }

    const handleViewDisk = () => {
        if (!file) return

        saveEpochRef.current += 1
        clearTimeout(mirrorTimeoutRef.current)
        pendingMirrorRef.current = false
        draftRef.current = file.content
        setSyncedContent(file.content)
        setPreviewSource(null)
        setDirty(false)
        setTabDirty({ tabId, dirty: false })
        setRestoreNotice('none')
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
        draftRef.current = mirror.content
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
     * with a current epoch) retries it against `draftRef.current` instead of the attempt being
     * silently dropped.
     */
    useEffect(() => {
        const flush = async () => {
            clearTimeout(mirrorTimeoutRef.current)
            if (!pendingMirrorRef.current || draftRef.current === null) return
            const committed = await persistMirror(draftRef.current, saveEpochRef.current).catch(() => false)
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

    useEffect(() => () => clearTimeout(autoSaveTimeoutRef.current), [])

    return {
        draftRef,
        dirty,
        setDirty,
        restoreNotice,
        setRestoreNotice,
        handleChange,
        handleSave,
        handleViewDisk,
        handleKeepMine,
        settleAfterDiskWrite,
    }
}
