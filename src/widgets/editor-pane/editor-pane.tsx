import type { FC } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Columns2 } from 'lucide-react'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import type { BlameLine, ConflictSides, FileSizeTier, HunkKind, MirrorEntry, ProjectId, TabId } from '@shared/api/bindings'
import { resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { monaco } from '@shared/lib/monaco/setup'
import { formatBlameLine } from '@shared/lib/blame-format'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { resolveSelectedTextOrCurrentLine } from '@shared/lib/editor-selection'
import { renderMarkdownToSafeHtml } from '@shared/lib/markdown'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { getStoredDiagnostics } from '@shared/lib/lsp/adapters/diagnostics'
import { applyCodeActionOrCommand, requestCodeActionsForKind, supportsCodeActionResolve } from '@shared/lib/lsp/adapters/code-action'
import { consumeExternallyDirtyModel } from '@shared/lib/lsp/model-dirty-tracker'
import { DEFAULT_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { HOT_EXIT_MIRROR_DEBOUNCE_MS } from '@shared/constants/mirror'
import { QUERY_KEY } from '@shared/constants/query-key'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { aiTokenStatusQueryOptions } from '@entities/ai/ai.query'
import { fileMirrorsQueryOptions, fileQueryOptions, useSaveFile } from '@entities/file/file.query'
import { clearMirror, mirrorDirty } from '@entities/file/file.ipc'
import { useSetTabDirty } from '@entities/layout/layout.query'
import { getGitBlameRange, getGitConflictSides } from '@entities/git/git.ipc'
import { OPEN_FILE_HISTORY_MONACO_ACTION_ID, TOGGLE_BLAME_MONACO_ACTION_ID } from '@entities/git/git.constant'
import { requestOpenFileHistory } from '@shared/lib/file-history-panel-bridge'
import { ideStatusQueryOptions } from '@entities/ide/ide.query'
import { systemOpenPath } from '@entities/system/system.ipc'
import { clearIdeSelection, setIdeSelection } from '@entities/ide/ide.ipc'
import {
    gitCurrentUserQueryOptions,
    gitGutterQueryOptions,
    gitStatusQueryOptions,
    useDiscardGitHunk,
    useResolveGitConflict,
    useStageGitHunk,
    useStageGitLines,
} from '@entities/git/git.query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import {
    acceptBothChanges,
    acceptCurrentChange,
    acceptIncomingChange,
    parseConflictMarkers,
    type ConflictRegion,
} from '@features/git/conflict-marker'
import { ConflictCompareDialog } from '@features/git/conflict-compare-dialog'
import { ConflictResolutionDialog } from '@features/git/conflict-resolution-dialog'
import { resolveSelectedLineRange } from '@features/git/selection-line-range'
import { HunkDiscardDialog } from '@features/git/hunk-discard-dialog'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { applyExternalContent } from '@entities/editor/model-registry'
import { registerMirrorFlush, unregisterMirrorFlush } from '@entities/editor/mirror-flush-registry'
import { consumePendingReveal } from '@entities/editor/reveal-registry'
import type { EditorCursorBlinkingStyle, EditorCursorStyle, EditorRenderWhitespace } from '@features/editor/code-editor'
import { CodeEditor } from '@features/editor/code-editor'
import { BlameFooterBar } from '@features/editor/blame-footer-bar'
import type { ConflictBannerVariant } from '@features/editor/conflict-banner'
import { ConflictBanner } from '@features/editor/conflict-banner'
import { MarkdownPreview } from '@features/editor/markdown-preview'
import { PaneSeparator } from '@features/split/pane-separator'
import { Button } from '@shared/ui/button'
import { isLspAttachableTier, useLspSession } from '@widgets/editor-pane/use-lsp-session'
import { peekLspSession, waitForLspSession } from '@widgets/editor-pane/lsp-session-registry'
import { BreadcrumbsBar } from '@widgets/editor-pane/breadcrumbs-bar'
import { isPathConflicted } from '@widgets/editor-pane/conflict-status'

const BLAME_DEBOUNCE_MS = 300
const MARKDOWN_PREVIEW_DEBOUNCE_MS = 200
const IDE_SELECTION_PUSH_DEBOUNCE_MS = 300
const MARKDOWN_LANGUAGE_ID = 'markdown'
const FORMAT_DOCUMENT_ACTION_ID = 'editor.action.formatDocument'
const TOGGLE_PREVIEW_BUTTON_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

/** LSP `CodeActionKind`s the two on-save booleans (`settings.fixAllOnSave`/`organizeImportsOnSave`) map to. */
const CODE_ACTION_KIND_FIX_ALL = 'source.fixAll'
const CODE_ACTION_KIND_ORGANIZE_IMPORTS = 'source.organizeImports'

/**
 * Upper bound on how long Code Actions on Save may block the save. VS Code has no hard timeout
 * here either (only a cancellable progress notification) — this project's decision (contract
 * §3.2) is a hard cap: past it, skip waiting further (already-applied edits stay; anything still
 * in flight lands whenever it lands) and warn instead of stalling an explicit ⌘S indefinitely.
 */
const CODE_ACTIONS_ON_SAVE_TIMEOUT_MS = 5_000

const DEFAULT_EDITOR_TAB_SIZE = 4
const DEFAULT_EDITOR_RENDER_WHITESPACE: EditorRenderWhitespace = 'selection'
const DEFAULT_EDITOR_CURSOR_STYLE: EditorCursorStyle = 'line'
const DEFAULT_EDITOR_CURSOR_BLINKING: EditorCursorBlinkingStyle = 'blink'

const GUTTER_CLASS_BY_HUNK_KIND: Record<HunkKind, string> = {
    added: 'taide-gutter-added',
    modified: 'taide-gutter-modified',
    deleted: 'taide-gutter-deleted',
}

const BLAME_OVERLAY_INLINE_CLASS_NAME = 'taide-blame-overlay-text'
const BLAME_OVERLAY_TEXT_PREFIX = '    '

type EditorPaneProps = {
    projectId: ProjectId
    tabId: TabId
    path: string
}

export const EditorPane: FC<EditorPaneProps> = ({ projectId, tabId, path }) => {
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
    const blameTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const previewTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const autoSaveTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const selectionTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const blameRequestSeqRef = useRef(0)
    const blameFooterTextRef = useRef<HTMLSpanElement>(null)
    /**
     * Mirrors the `path` prop — kept fresh by the `[path]` effect below rather than read directly,
     * so a stale closure can tell "still the same path" apart from "armed for a path this instance
     * has since moved on from". `EditorPane` has no `key` (see the mirror-cache doc comment
     * further below), so switching tabs in the same pane reuses this instance and its in-flight
     * timers rather than remounting them away; `handleChange`'s auto-save and markdown-preview
     * timers check this at fire time against the `scheduledPath` they captured when armed.
     */
    const pathRef = useRef(path)

    const [syncedPath, setSyncedPath] = useState(path)
    const [syncedContent, setSyncedContent] = useState<string | null>(null)
    const [dirty, setDirty] = useState(false)
    const [editor, setEditor] = useState<monaco.editor.IStandaloneCodeEditor | null>(null)
    const [cursorLine, setCursorLine] = useState<number | null>(null)
    const [pendingHunk, setPendingHunk] = useState<{ start: number; end: number } | null>(null)
    const [blameLine, setBlameLine] = useState<BlameLine | null>(null)
    const [blameOverlayEnabled, setBlameOverlayEnabled] = useState(false)
    const [blameOverlayLines, setBlameOverlayLines] = useState<BlameLine[] | null>(null)
    const [showMarkdownPreview, setShowMarkdownPreview] = useState(false)
    const [previewSource, setPreviewSource] = useState<string | null>(null)
    const [restoreNotice, setRestoreNotice] = useState<Exclude<ConflictBannerVariant, 'changedOnDisk'> | 'none'>('none')
    const [conflictRegions, setConflictRegions] = useState<ConflictRegion[]>([])
    const [pendingConflict, setPendingConflict] = useState<ConflictRegion | null>(null)
    const [compareSides, setCompareSides] = useState<ConflictSides | null>(null)

    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const { data: file, isPending, isError, error } = useQuery(fileQueryOptions(path))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { data: ideStatus } = useQuery(ideStatusQueryOptions())
    const { data: gutterHunks } = useQuery(gitGutterQueryOptions({ projectId, path }))
    const { data: mirrors } = useQuery(fileMirrorsQueryOptions(projectId))
    const { data: lspServers } = useQuery(lspServersQueryOptions())
    const { mutate: discardHunk } = useDiscardGitHunk(projectId)
    const { data: currentUser } = useQuery(gitCurrentUserQueryOptions(projectId))
    const { mutate: saveFile } = useSaveFile()
    const { mutate: setTabDirty } = useSetTabDirty(projectId)
    const { mutate: updateSettings } = useUpdateSettings()
    const { data: gitStatus } = useQuery(gitStatusQueryOptions(projectId))
    const { mutate: resolveConflict } = useResolveGitConflict(projectId)
    const { mutate: stageHunk } = useStageGitHunk(projectId)
    const { mutate: stageLines } = useStageGitLines(projectId)

    if (path !== syncedPath) {
        setSyncedPath(path)
        setSyncedContent(null)
        setPreviewSource(null)
        setDirty(false)
        setBlameLine(null)
        setBlameOverlayEnabled(false)
        setRestoreNotice('none')
    } else if (file && syncedContent === null) {
        setSyncedContent(file.content)
    } else if (file && !dirty && syncedContent !== null && file.content !== syncedContent) {
        setSyncedContent(file.content)
        setPreviewSource(null)
    }

    const conflict = dirty && syncedContent !== null && !!file && file.content !== syncedContent
    const isMarkdown = file?.languageId === MARKDOWN_LANGUAGE_ID
    const isConflicted = isPathConflicted(gitStatus?.rows ?? [], path)

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
        const autoSaveDelayMs = settings?.autoSaveDelayMs ?? 0
        clearTimeout(autoSaveTimeoutRef.current)
        if (autoSaveDelayMs > 0 && !savingRef.current) {
            autoSaveTimeoutRef.current = setTimeout(() => {
                if (pathRef.current === scheduledPath) void handleSave('auto')
            }, autoSaveDelayMs)
        }

        if (!isMarkdown) return
        clearTimeout(previewTimeoutRef.current)
        previewTimeoutRef.current = setTimeout(() => {
            if (pathRef.current === scheduledPath) setPreviewSource(value)
        }, MARKDOWN_PREVIEW_DEBOUNCE_MS)
    }

    /**
     * LSP servers attached for `languageId` — must stay in exact lockstep with `use-lsp-session.ts`'s
     * own attach gate (language-matching, installed/available, *and* `isLspAttachableTier`), not
     * just approximate it: a serverId this returns but `use-lsp-session` would never actually
     * attach a session for (e.g. a large/read-only-tier file) makes `waitForLspSession` below wait
     * on a session that will never be created.
     */
    const matchingLspServerIds = (languageId: string, tier: FileSizeTier | null) =>
        isLspAttachableTier(tier)
            ? (lspServers ?? []).filter((server) => server.languageIds.includes(languageId) && server.available).map((server) => server.id)
            : []

    /**
     * Sends `textDocument/didSave` to every already-attached session for this file once the disk
     * write succeeds — LSP servers that key diagnostics/state off save (not just in-memory
     * `didChange`) need this notification to fire at all. Uses `peekLspSession` (no waiting)
     * rather than `waitForLspSession`: there is nothing to notify a session that was never
     * attached in the first place, and waiting for one to *become* attached here would just leak
     * an unresolved waiter for every save on a file with no matching session.
     */
    const notifyLspSessionsOfSave = async () => {
        const languageId = file?.languageId
        if (!languageId) return
        const uri = monaco.Uri.file(path).toString()

        await Promise.all(
            matchingLspServerIds(languageId, file?.tier ?? null).map(async (serverId) => {
                const session = peekLspSession(projectId, serverId)
                const ready = await session?.ready.catch(() => null)
                ready?.client.didSave(uri)
            }),
        )
    }

    /**
     * Runs `source.fixAll` then `source.organizeImports` (fixAll first, matching VS Code's own
     * save-participant ordering) against every attached, code-action-capable LSP session for this
     * file, applying whatever each returns via {@link applyCodeActionOrCommand}. Bypasses monaco's
     * `CodeActionController` entirely — it drives the client directly, the same way the outline
     * panel does for `textDocument/documentSymbol` — because `editor.action.organizeImports`'s
     * `run()` does not await the edit being applied (confirmed against monaco's source), so it
     * cannot be used to sequence code-action → format → save deterministically. Bounded by
     * {@link CODE_ACTIONS_ON_SAVE_TIMEOUT_MS}; a timeout stops *waiting* (format/save proceed with
     * whatever already landed) rather than aborting the underlying LSP requests.
     */
    const runCodeActionsOnSave = async () => {
        const languageId = file?.languageId
        const model = editor?.getModel()
        if (!languageId || !model) return

        const kinds = [
            settings?.fixAllOnSave && CODE_ACTION_KIND_FIX_ALL,
            settings?.organizeImportsOnSave && CODE_ACTION_KIND_ORGANIZE_IMPORTS,
        ].filter((kind): kind is string => Boolean(kind))
        if (kinds.length === 0) return

        const serverIds = matchingLspServerIds(languageId, file?.tier ?? null)
        if (serverIds.length === 0) return

        const uri = model.uri.toString()
        const range = monacoRangeToLsp(model.getFullModelRange())

        /**
         * Cancel handles for every `waitForLspSession` waiter registered below, invoked only if
         * the overall timeout fires — a waiter left pending past that point (e.g. a session whose
         * spawn/initialize never settles) would otherwise sit in `waitersByKey` forever, growing by
         * one every time this runs. On the normal (non-timeout) path each waiter already resolves
         * and self-removes, so there is nothing to clean up there.
         */
        const pendingCancels: (() => void)[] = []
        const applyAllKinds = (async () => {
            for (const serverId of serverIds) {
                const waiter = waitForLspSession(projectId, serverId)
                pendingCancels.push(waiter.cancel)
                const session = await waiter.promise
                const ready = await session?.ready.catch(() => null)
                if (!ready) continue

                const supportsResolve = supportsCodeActionResolve(ready.client)
                const diagnostics = getStoredDiagnostics(serverId, uri)
                for (const kind of kinds) {
                    const actions = await requestCodeActionsForKind(ready.client, uri, range, diagnostics, kind).catch(() => [])
                    for (const action of actions) await applyCodeActionOrCommand(monaco, ready.client, supportsResolve, action).catch(() => undefined)
                }
            }
        })()

        let timeoutId: ReturnType<typeof setTimeout> | undefined
        const timedOut = await Promise.race([
            applyAllKinds.then(() => false),
            new Promise<boolean>((resolve) => (timeoutId = setTimeout(() => resolve(true), CODE_ACTIONS_ON_SAVE_TIMEOUT_MS))),
        ])
        clearTimeout(timeoutId)
        if (timedOut) {
            pendingCancels.forEach((cancel) => cancel())
            toast.error(t('editor.codeActionsOnSaveSkipped'))
        }
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

        if (settings?.formatOnSave) {
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
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
                    void notifyLspSessionsOfSave()
                },
                onError: (saveError) => {
                    savingRef.current = false
                    toast.error(saveError.message)
                },
            },
        )
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

    const handleMinimapToggle = (enabled: boolean) => updateSettings({ ...emptySettingsPatch(), editorMinimap: enabled })

    const handleEditorMount = (nextEditor: monaco.editor.IStandaloneCodeEditor | null) => setEditor(nextEditor)

    /**
     * Applies one side's transform to `region` as a single undoable `executeEdits` op (replacing
     * the whole buffer via `getFullModelRange` rather than hand-computing the region's own range —
     * simpler and avoids edge cases around the region sitting at the very end of the file). Once
     * the resulting content has no conflict markers left at all — not just none in `region` — the
     * file is fully resolved, so `git_resolve_conflict` writes it to disk and re-stages it in the
     * same step. A resolve while markers remain elsewhere would incorrectly clear the index's
     * unmerged entry for a file that still has unresolved regions.
     *
     * `executeEdits` fires `onDidChangeModelContent` synchronously, so by the time it returns
     * `handleChange` has already marked the tab dirty and armed a mirror write for `newContent` —
     * both of which `git_resolve_conflict`'s own write to disk (below) makes redundant once it
     * succeeds. `onSuccess` clears them the same way `handleSave`'s own success handler does, plus
     * invalidates `FILE.CONTENT` so the query cache catches up to what's now on disk.
     */
    const applyConflictResolution = (region: ConflictRegion, transform: (content: string, target: ConflictRegion) => string) => {
        setPendingConflict(null)
        const model = editor?.getModel()
        if (!model) return
        const newContent = transform(model.getValue(), region)
        editor?.executeEdits('taide.conflictResolution', [{ range: model.getFullModelRange(), text: newContent }])
        if (parseConflictMarkers(newContent).length > 0) return
        resolveConflict(
            { projectId, path, content: newContent },
            {
                onSuccess: () => {
                    saveEpochRef.current += 1
                    clearTimeout(mirrorTimeoutRef.current)
                    pendingMirrorRef.current = false
                    setDirty(false)
                    setTabDirty({ tabId, dirty: false })
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
                    toast.success(t('git.conflictResolved'))
                },
                onError: (mutationError) => toast.error(mutationError.message),
            },
        )
    }

    const handleAcceptCurrentChange = () => pendingConflict && applyConflictResolution(pendingConflict, acceptCurrentChange)
    const handleAcceptIncomingChange = () => pendingConflict && applyConflictResolution(pendingConflict, acceptIncomingChange)
    const handleAcceptBothChanges = () => pendingConflict && applyConflictResolution(pendingConflict, acceptBothChanges)

    const handleCompareConflict = () => {
        setPendingConflict(null)
        void getGitConflictSides({ projectId, path })
            .then(setCompareSides)
            .catch((compareError: Error) => toast.error(compareError.message))
    }

    useLspSession({
        projectId,
        path,
        languageId: file?.languageId ?? null,
        tier: file?.tier ?? null,
        enabled: !isPending && !isError && file?.tier !== 'refused',
    })

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

    /**
     * Registers the mounted monaco instance under `tabId` for the shared editor-instance registry
     * (breadcrumbs, the status bar, and `editor-area`'s own lookups all read it). Keyed by
     * `[tabId, editor]` rather than done once inside `handleEditorMount` — `CodeEditor`'s own
     * mount effect has an empty dependency array (it swaps buffers via `setModel`, never
     * remounting monaco), and this pane has no `key` either, so `handleEditorMount` fires exactly
     * once for the pane's whole lifetime while the same instance goes on to serve every tab the
     * user switches to in this pane. Without this effect, the registry stayed pinned to whichever
     * tab happened to be active at first mount; every later tab switch left it stale, and
     * `getEditorInstance` for the now-active tab returned nothing (or a leftover instance for a
     * tab that no longer owns it). Re-running on every `tabId` change re-keys the same live
     * instance instead.
     */
    useEffect(() => {
        if (!editor) return
        registerEditorInstance(tabId, editor)
        return () => unregisterEditorInstance(tabId)
    }, [tabId, editor])

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

    useEffect(() => () => clearTimeout(previewTimeoutRef.current), [])

    /**
     * Before syncing this tab's model to the last-known disk content, checks whether an LSP
     * `WorkspaceEdit` landed on this model while it had no mounted editor watching it (a
     * background tab in another pane — see `model-dirty-tracker.ts`). If so, that edit is the
     * model's current value and must become this tab's dirty draft instead of being silently
     * overwritten by `applyExternalContent`, which cannot otherwise tell "never diverged from
     * disk" apart from "diverged via an edit this component never observed". Deferred to a
     * microtask (matching `applyMirrorRestore` above) since it calls `setState`, which an effect
     * body must not do synchronously.
     */
    const syncModelOrPickUpExternalEdit = useEffectEvent(() => {
        if (!editor || syncedContent === null || dirty) return
        if (consumeExternallyDirtyModel(path)) {
            const model = editor.getModel()
            if (!model) return
            draftRef.current = model.getValue()
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
            return
        }
        applyExternalContent(path, syncedContent, editor)
    })

    useEffect(() => {
        if (!editor || syncedContent === null || dirty) return
        queueMicrotask(syncModelOrPickUpExternalEdit)
    }, [editor, syncedContent, dirty, path])

    useEffect(() => {
        if (!editor) return
        consumePendingReveal(path, editor)
    }, [editor, path])

    /**
     * Re-parses conflict markers out of the live model on every content change (not just the
     * initial `file.content` snapshot) — resolving one region via `applyConflictResolution` edits
     * the model directly, and the decorations/dialog below must reflect the remaining regions
     * immediately, without waiting on a git-status refetch. Every consumer of `conflictRegions`
     * already gates on `isConflicted` itself, so there is nothing to reset when it goes false —
     * the stale array is simply never read. The initial parse is deferred to a microtask rather
     * than called synchronously in the effect body (an effect body must not call `setState`
     * synchronously — matches the same constraint `applyMirrorRestore`'s caller works around above).
     */
    useEffect(() => {
        if (!editor || !isConflicted) return
        const model = editor.getModel()
        if (!model) return
        const parseAndSetConflictRegions = () => setConflictRegions(parseConflictMarkers(model.getValue()))
        queueMicrotask(parseAndSetConflictRegions)
        const subscription = editor.onDidChangeModelContent(parseAndSetConflictRegions)
        return () => subscription.dispose()
    }, [editor, path, isConflicted])

    useEffect(() => {
        if (!editor) return

        const decorations = isConflicted
            ? conflictRegions.flatMap((region) => [
                  {
                      range: new monaco.Range(region.startLine, 1, (region.baseLine ?? region.separatorLine) - 1, 1),
                      options: { className: 'taide-conflict-current-background', isWholeLine: true },
                  },
                  {
                      range: new monaco.Range(region.separatorLine, 1, region.endLine, 1),
                      options: { className: 'taide-conflict-incoming-background', isWholeLine: true },
                  },
                  {
                      range: new monaco.Range(region.startLine, 1, region.startLine, 1),
                      options: { linesDecorationsClassName: 'taide-conflict-gutter-action', isWholeLine: true },
                  },
              ])
            : (gutterHunks ?? []).map((hunk) => ({
                  range: new monaco.Range(hunk.start, 1, hunk.end, 1),
                  options: { linesDecorationsClassName: GUTTER_CLASS_BY_HUNK_KIND[hunk.kind], isWholeLine: true },
              }))
        const collection = editor.createDecorationsCollection(decorations)
        return () => collection.clear()
    }, [editor, gutterHunks, isConflicted, conflictRegions])

    /**
     * A conflicted file's gutter shows conflict-region markers instead of hunk bars (above), so a
     * click there must open {@link ConflictResolutionDialog} instead of the plain hunk-discard
     * flow — diffing this file against HEAD (what `gutterHunks` reflects) is meaningless while it
     * still has unresolved conflict markers.
     */
    useEffect(() => {
        if (!editor) return
        const subscription = editor.onMouseDown((event) => {
            if (event.target.type !== monaco.editor.MouseTargetType.GUTTER_LINE_DECORATIONS) return
            const line = event.target.position?.lineNumber
            if (!line) return
            if (isConflicted) {
                const region = conflictRegions.find((candidate) => candidate.startLine === line)
                if (region) setPendingConflict(region)
                return
            }
            const hunk = (gutterHunks ?? []).find((candidate) => line >= candidate.start && line <= candidate.end)
            if (!hunk) return
            setPendingHunk({ start: hunk.start, end: hunk.end })
        })
        return () => subscription.dispose()
    }, [editor, gutterHunks, isConflicted, conflictRegions])

    /**
     * Registers "Stage Changes" in the editor's right-click context menu — `run()` reads the
     * *current* selection/cursor at invocation time rather than reacting to selection-change
     * events. A non-empty selection stages exactly those lines (`resolveSelectedLineRange` trims a
     * trailing line the selection only touches at column 1), while an empty selection (just a
     * cursor) falls back to the whole hunk containing that line. No-ops while the file is still
     * conflicted — `gutterHunks`' workdir-relative hunks don't correspond to anything
     * `git_stage_hunk` can act on until the markers are resolved.
     *
     * There is deliberately no "Unstage Changes" counterpart here. `gutterHunks` reports hunks in
     * *workdir* coordinates (`git_gutter` diffs HEAD against the workdir), which only line up with
     * `git_unstage_hunk`/`git_unstage_lines`'s *index*-relative matching when the file's staged and
     * unstaged regions are identical — i.e. never for a partially staged file, which is exactly the
     * case unstage exists to serve. Unstage is offered instead on the indexVsHead diff tab
     * (`DiffPane` with `staged=true`, opened from the git panel's "Staged Changes" group), whose
     * hunk ranges are already index-relative and match the backend one-to-one.
     */
    useEffect(() => {
        if (!editor) return

        const stageSelection = (targetEditor: monaco.editor.ICodeEditor) => {
            if (isConflicted) return
            const selection = targetEditor.getSelection()
            if (!selection) return
            const onError = (mutationError: Error) => toast.error(mutationError.message)

            if (!selection.isEmpty()) {
                const { start, end } = resolveSelectedLineRange(selection)
                stageLines({ projectId, path, lineStart: start, lineEnd: end }, { onError })
                return
            }

            const line = selection.startLineNumber
            const hunk = (gutterHunks ?? []).find((candidate) => line >= candidate.start && line <= candidate.end)
            if (!hunk) return
            stageHunk({ projectId, path, hunkStart: hunk.start, hunkEnd: hunk.end }, { onError })
        }

        const stageAction = editor.addAction({
            id: 'taide.gitStageSelection',
            label: t('git.stageChanges'),
            contextMenuGroupId: '9_git',
            contextMenuOrder: 1,
            run: stageSelection,
        })

        return () => stageAction.dispose()
    }, [editor, t, isConflicted, gutterHunks, projectId, path, stageHunk, stageLines])

    useEffect(() => {
        if (!editor || !ideStatus?.running) return

        const subscription = editor.onDidChangeCursorSelection((event) => {
            clearTimeout(selectionTimeoutRef.current)
            selectionTimeoutRef.current = setTimeout(() => {
                const model = editor.getModel()
                if (!model) return
                const range = monacoRangeToLsp(event.selection)
                void setIdeSelection({
                    projectId,
                    path,
                    text: model.getValueInRange(event.selection),
                    startLine: range.start.line,
                    startCharacter: range.start.character,
                    endLine: range.end.line,
                    endCharacter: range.end.character,
                    isEmpty: event.selection.isEmpty(),
                }).catch(() => undefined)
            }, IDE_SELECTION_PUSH_DEBOUNCE_MS)
        })

        return () => {
            subscription.dispose()
            clearTimeout(selectionTimeoutRef.current)
            void clearIdeSelection().catch(() => undefined)
        }
    }, [editor, ideStatus?.running, projectId, path])

    useEffect(() => {
        if (!editor || cursorLine === null) return

        clearTimeout(blameTimeoutRef.current)
        blameTimeoutRef.current = setTimeout(() => {
            const requestSeq = ++blameRequestSeqRef.current
            void getGitBlameRange({ projectId, path, from: cursorLine, to: cursorLine })
                .then((lines) => {
                    if (blameRequestSeqRef.current !== requestSeq) return
                    setBlameLine(lines[0] ?? null)
                })
                .catch(() => {
                    if (blameRequestSeqRef.current !== requestSeq) return
                    setBlameLine(null)
                })
        }, BLAME_DEBOUNCE_MS)

        return () => clearTimeout(blameTimeoutRef.current)
    }, [editor, cursorLine, projectId, path])

    useEffect(() => {
        const node = blameFooterTextRef.current
        if (!node) return

        const model = editor?.getModel() ?? null
        node.textContent = !blameLine || (model && blameLine.line > model.getLineCount()) ? '' : formatBlameLine(blameLine, Date.now(), currentUser)
    }, [editor, blameLine, currentUser])

    useEffect(() => {
        if (!editor || !blameOverlayEnabled) return
        const model = editor.getModel()
        if (!model) return

        let cancelled = false
        void getGitBlameRange({ projectId, path, from: 1, to: model.getLineCount() })
            .then((lines) => {
                if (!cancelled) setBlameOverlayLines(lines)
            })
            .catch(() => {
                if (!cancelled) setBlameOverlayLines(null)
            })
        return () => {
            cancelled = true
        }
    }, [editor, blameOverlayEnabled, projectId, path])

    useEffect(() => {
        if (!editor || !blameOverlayEnabled || !blameOverlayLines) return
        const model = editor.getModel()
        if (!model) return

        const decorations = blameOverlayLines
            .filter((line) => line.line <= model.getLineCount())
            .map((line) => ({
                range: new monaco.Range(line.line, model.getLineMaxColumn(line.line), line.line, model.getLineMaxColumn(line.line)),
                options: {
                    after: {
                        content: `${BLAME_OVERLAY_TEXT_PREFIX}${formatBlameLine(line, Date.now(), currentUser)}`,
                        inlineClassName: BLAME_OVERLAY_INLINE_CLASS_NAME,
                    },
                },
            }))
        const collection = editor.createDecorationsCollection(decorations)
        return () => collection.clear()
    }, [editor, blameOverlayEnabled, blameOverlayLines, currentUser])

    useEffect(() => {
        if (!editor) return
        const action = editor.addAction({
            id: TOGGLE_BLAME_MONACO_ACTION_ID,
            label: t('git.toggleBlame'),
            contextMenuGroupId: 'navigation',
            run: () => setBlameOverlayEnabled((previous) => !previous),
        })
        return () => action.dispose()
    }, [editor, t])

    useEffect(() => {
        if (!editor) return
        const action = editor.addAction({
            id: OPEN_FILE_HISTORY_MONACO_ACTION_ID,
            label: t('git.fileHistory'),
            contextMenuGroupId: '9_git',
            contextMenuOrder: 3,
            run: () => requestOpenFileHistory(path),
        })
        return () => action.dispose()
    }, [editor, t, path])

    useEffect(() => {
        if (!editor) return
        const action = editor.addAction({
            id: 'taide.runSelectedTextInTerminal',
            label: t('terminal.runSelectedText'),
            contextMenuGroupId: '9_terminal',
            run: (targetEditor) => {
                const text = resolveSelectedTextOrCurrentLine(targetEditor)
                if (text !== null) requestEditorPaneCommand({ type: 'run-in-terminal', text, cwd: null })
            },
        })
        return () => action.dispose()
    }, [editor, t])

    if (isPending) return <div className='bg-editor-background h-full w-full' />

    if (isError) {
        return (
            <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>
                {error instanceof Error ? error.message : t('editor.openFailed')}
            </div>
        )
    }

    if (file.tier === 'refused') {
        return (
            <div className='bg-editor-background text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-2 text-sm'>
                <span>{t('editor.cannotOpen')}</span>
                <span className='text-xs opacity-70'>{t('editor.binaryOrTooLarge')}</span>
                <Button
                    type='button'
                    variant='outline'
                    size='xs'
                    onClick={() => void systemOpenPath(path).catch((error: Error) => toast.error(error.message))}>
                    {t('editor.openExternally')}
                </Button>
            </div>
        )
    }

    const aiCompletionConfig = resolveAiInlineCompletionConfig(settings, aiTokenStatus)

    const codeEditor = (
        <CodeEditor
            path={file.path}
            language={file.languageId}
            value={file.content}
            readOnly={file.readOnly}
            largeFile={file.tier === 'large' || file.tier === 'readOnly'}
            fontFamily={buildMonospaceFontStack(settings?.editorFontFamily ?? null)}
            fontSize={settings?.editorFontSize ?? DEFAULT_CODE_FONT_SIZE}
            minimap={settings?.editorMinimap ?? true}
            wordWrap={settings?.editorWordWrap ?? false}
            lineNumbers={settings?.editorLineNumbers ?? true}
            tabSize={settings?.editorTabSize ?? DEFAULT_EDITOR_TAB_SIZE}
            insertSpaces={settings?.editorInsertSpaces ?? true}
            detectIndentation={settings?.editorDetectIndentation ?? true}
            renderWhitespace={settings?.editorRenderWhitespace ?? DEFAULT_EDITOR_RENDER_WHITESPACE}
            bracketPairColorization={settings?.editorBracketPairColorization ?? true}
            fontLigatures={settings?.editorFontLigatures ?? false}
            cursorStyle={settings?.editorCursorStyle ?? DEFAULT_EDITOR_CURSOR_STYLE}
            cursorBlinking={settings?.editorCursorBlinking ?? DEFAULT_EDITOR_CURSOR_BLINKING}
            scrollBeyondLastLine={settings?.editorScrollBeyondLastLine ?? true}
            stickyScroll={settings?.editorStickyScrollEnabled ?? true}
            formatOnType={settings?.editorFormatOnType ?? false}
            formatOnPaste={settings?.editorFormatOnPaste ?? false}
            aiAutoTabEnabled={settings?.aiAutoTabEnabled ?? false}
            aiCompletionConfig={aiCompletionConfig}
            onChange={handleChange}
            onSave={handleSave}
            onCursorLineChange={setCursorLine}
            onEditorMount={handleEditorMount}
            onMinimapToggle={handleMinimapToggle}
        />
    )

    const codeEditorWithBlameFooter = (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <div className='min-h-0 flex-1'>{codeEditor}</div>
            <BlameFooterBar textRef={blameFooterTextRef} />
        </div>
    )

    const handleConfirmDiscardHunk = () => {
        if (!pendingHunk) return
        discardHunk(
            { projectId, path, hunkStart: pendingHunk.start, hunkEnd: pendingHunk.end },
            { onError: (mutationError) => toast.error(mutationError.message) },
        )
        setPendingHunk(null)
    }

    const bannerVariant: ConflictBannerVariant | 'none' = conflict ? 'changedOnDisk' : restoreNotice

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <HunkDiscardDialog
                startLine={pendingHunk?.start ?? null}
                endLine={pendingHunk?.end ?? null}
                onCancel={() => setPendingHunk(null)}
                onConfirm={handleConfirmDiscardHunk}
            />
            <ConflictResolutionDialog
                region={pendingConflict}
                onCancel={() => setPendingConflict(null)}
                onAcceptCurrent={handleAcceptCurrentChange}
                onAcceptIncoming={handleAcceptIncomingChange}
                onAcceptBoth={handleAcceptBothChanges}
                onCompare={handleCompareConflict}
            />
            <ConflictCompareDialog sides={compareSides} languageId={file.languageId} onOpenChange={(open) => !open && setCompareSides(null)} />
            <BreadcrumbsBar projectId={projectId} tabId={tabId} path={path} />
            {file.readOnly && (
                <div className='bg-status-warning/15 text-status-warning shrink-0 px-3 py-1 text-xs'>{t('editor.readOnlyLargeFile')}</div>
            )}
            {bannerVariant !== 'none' && (
                <ConflictBanner
                    variant={bannerVariant}
                    onViewDisk={handleViewDisk}
                    onKeepMine={handleKeepMine}
                    onDismiss={() => setRestoreNotice('none')}
                />
            )}
            {isMarkdown && (
                <div className='border-app-border flex h-8 shrink-0 items-center justify-end border-b px-2'>
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <button
                                type='button'
                                aria-pressed={showMarkdownPreview}
                                aria-label={t('editor.toggleMarkdownPreview')}
                                onClick={() => setShowMarkdownPreview((previous) => !previous)}
                                className={TOGGLE_PREVIEW_BUTTON_CLASS}>
                                <Columns2 className='size-4' />
                            </button>
                        </TooltipTrigger>
                        <TooltipContent side='bottom'>{t('editor.toggleMarkdownPreview')}</TooltipContent>
                    </Tooltip>
                </div>
            )}
            {isMarkdown && showMarkdownPreview ? (
                <Group orientation='horizontal' className='min-h-0 min-w-0 flex-1'>
                    <Panel id={`${tabId}-editor`} defaultSize='50%' minSize='20%' className='min-h-0 min-w-0'>
                        {codeEditorWithBlameFooter}
                    </Panel>
                    <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                    <Panel id={`${tabId}-preview`} defaultSize='50%' minSize='20%' className='min-h-0 min-w-0'>
                        <MarkdownPreview html={renderMarkdownToSafeHtml(previewSource ?? syncedContent ?? file.content)} />
                    </Panel>
                </Group>
            ) : (
                codeEditorWithBlameFooter
            )}
        </div>
    )
}
