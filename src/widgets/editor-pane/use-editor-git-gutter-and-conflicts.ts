import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { useTranslation } from 'react-i18next'
import type { ConflictSides, HunkKind, ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { resolveSelectedLineRange } from '@shared/lib/selection-line-range'
import { requestOpenFileHistory } from '@shared/lib/file-history-panel-bridge'
import {
    gitConflictSidesQueryOptions,
    gitGutterQueryOptions,
    gitStatusQueryOptions,
    useDiscardGitHunk,
    useResolveGitConflict,
    useStageGitHunk,
    useStageGitLines,
} from '@entities/git/git.query'
import { OPEN_FILE_HISTORY_MONACO_ACTION_ID } from '@entities/git/git.constant'
import {
    acceptBothChanges,
    acceptCurrentChange,
    acceptIncomingChange,
    parseConflictMarkers,
    type ConflictRegion,
} from '@features/git/conflict-marker'
import { isPathConflicted } from '@widgets/editor-pane/conflict-status'

const GUTTER_CLASS_BY_HUNK_KIND: Record<HunkKind, string> = {
    added: 'taide-gutter-added',
    modified: 'taide-gutter-modified',
    deleted: 'taide-gutter-deleted',
}

type UseEditorGitGutterAndConflictsInput = {
    projectId: ProjectId
    path: string
    editor: monaco.editor.IStandaloneCodeEditor | null
    t: ReturnType<typeof useTranslation>['t']
    settleAfterDiskWrite: () => void
}

/**
 * Owns `EditorPane`'s git gutter (hunk decorations, discard, stage-from-selection) and unresolved
 * merge-conflict handling (region decorations, the resolution dialog, compare-sides). The two are
 * combined in one hook because they share the same gutter-click and decoration effects — a
 * conflicted file's gutter renders conflict-region markers instead of hunk bars, and a gutter
 * click opens the conflict dialog instead of the hunk-discard flow.
 */
export const useEditorGitGutterAndConflicts = ({ projectId, path, editor, t, settleAfterDiskWrite }: UseEditorGitGutterAndConflictsInput) => {
    const [pendingHunk, setPendingHunk] = useState<{ start: number; end: number } | null>(null)
    const [conflictRegions, setConflictRegions] = useState<ConflictRegion[]>([])
    const [pendingConflict, setPendingConflict] = useState<ConflictRegion | null>(null)
    const [compareRequested, setCompareRequested] = useState(false)

    const { data: gutterHunks } = useQuery(gitGutterQueryOptions({ projectId, path }))
    const { data: gitStatus } = useQuery(gitStatusQueryOptions(projectId))
    const { mutate: discardHunk } = useDiscardGitHunk(projectId)
    const { mutate: resolveConflict } = useResolveGitConflict(projectId)
    const { mutate: stageHunk } = useStageGitHunk(projectId)
    const { mutate: stageLines } = useStageGitLines(projectId)

    const isConflicted = isPathConflicted(gitStatus?.rows ?? [], path)

    /**
     * Fetched on demand (`compareRequested`, armed by `handleCompareConflict` below) rather than
     * whenever the file happens to be conflicted — matches the pre-query imperative fetch this
     * replaces, which only ever ran in response to the compare dialog's own "Compare" button.
     * `refetch` is what makes every click of "Compare" actually retry: `enabled: compareRequested`
     * alone can't, because a click while already `compareRequested === true` (the dialog failed to
     * open on the previous attempt, so it's still armed) is a no-op `setState` that never re-renders,
     * let alone re-fetches — matching the pre-query `.then(setCompareSides).catch(toast.error)` this
     * replaces, which re-ran unconditionally on every click regardless of the previous attempt's
     * outcome.
     */
    const {
        data: compareSidesData,
        isError: isCompareSidesError,
        error: compareSidesErrorValue,
        refetch: refetchCompareSides,
    } = useQuery({ ...gitConflictSidesQueryOptions({ projectId, path }), enabled: compareRequested })
    const compareSides = compareRequested ? (compareSidesData ?? null) : null

    /**
     * Gated on `compareRequested` (not just `isCompareSidesError`) so a stale error cached from a
     * *previous* failed attempt — surfaced the instant this component mounts/re-renders for a path
     * that already has one, `enabled` or not — doesn't toast without the user having done anything.
     * Only an attempt actually armed by {@link handleCompareConflict} below reaches the user.
     */
    useEffect(() => {
        if (!compareRequested || !isCompareSidesError) return
        toast.error(compareSidesErrorValue instanceof Error ? compareSidesErrorValue.message : String(compareSidesErrorValue))
    }, [compareRequested, isCompareSidesError, compareSidesErrorValue])

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
     * succeeds. `onSuccess` clears them the same way `handleSave`'s own success handler does;
     * `useResolveGitConflict` itself (contract F3#4) owns invalidating `FILE.CONTENT`/`FILE.MIRRORS`
     * so the query cache catches up to what's now on disk.
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
                    settleAfterDiskWrite()
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
        setCompareRequested(true)
        void refetchCompareSides()
    }

    /**
     * `ConflictCompareDialog`'s `onOpenChange` only ever calls this with `null` (closing) —
     * preserved under its pre-query name/signature so `editor-pane.tsx` needed no change, but now
     * clears the on-demand fetch's request flag instead of a piece of state holding the fetched
     * value directly.
     */
    const setCompareSides = (sides: ConflictSides | null) => {
        if (sides === null) setCompareRequested(false)
    }

    const handleConfirmDiscardHunk = () => {
        if (!pendingHunk) return
        discardHunk(
            { projectId, path, hunkStart: pendingHunk.start, hunkEnd: pendingHunk.end },
            { onError: (mutationError) => toast.error(mutationError.message) },
        )
        setPendingHunk(null)
    }

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

    return {
        pendingHunk,
        setPendingHunk,
        handleConfirmDiscardHunk,
        pendingConflict,
        setPendingConflict,
        handleAcceptCurrentChange,
        handleAcceptIncomingChange,
        handleAcceptBothChanges,
        handleCompareConflict,
        compareSides,
        setCompareSides,
    }
}
