import { useEffect, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { useTranslation } from 'react-i18next'
import type { BlameLine, ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { formatBlameLine } from '@shared/lib/blame-format'
import { gitBlameLineQueryOptions, gitBlameOverlayQueryOptions, gitCurrentUserQueryOptions } from '@entities/git/git.query'
import { TOGGLE_BLAME_MONACO_ACTION_ID } from '@entities/git/git.constant'

const BLAME_DEBOUNCE_MS = 300
const BLAME_OVERLAY_INLINE_CLASS_NAME = 'taide-blame-overlay-text'
const BLAME_OVERLAY_TEXT_PREFIX = '    '

type UseEditorBlameInput = {
    projectId: ProjectId
    path: string
    editor: monaco.editor.IStandaloneCodeEditor | null
    t: ReturnType<typeof useTranslation>['t']
}

type DebouncedBlameLineQuery = { projectId: ProjectId; path: string; line: number } | null

/**
 * Owns `EditorPane`'s git-blame surfaces — the per-cursor-line blame footer, the toggleable
 * whole-file blame overlay decorations, and the monaco action that toggles the overlay.
 */
export const useEditorBlame = ({ projectId, path, editor, t }: UseEditorBlameInput) => {
    const blameFooterTextRef = useRef<HTMLSpanElement>(null)

    const [cursorLine, setCursorLine] = useState<number | null>(null)
    const [debouncedBlameQuery, setDebouncedBlameQuery] = useState<DebouncedBlameLineQuery>(null)
    const [blameLine, setBlameLine] = useState<BlameLine | null>(null)
    const [blameOverlayEnabled, setBlameOverlayEnabled] = useState(false)
    const [overlayLineCount, setOverlayLineCount] = useState<number | null>(null)

    const { data: currentUser } = useQuery(gitCurrentUserQueryOptions(projectId))

    /**
     * Arms a debounced fetch of the cursor's blame line {@link BLAME_DEBOUNCE_MS} after it last
     * moved — `debouncedBlameQuery` only advances once the debounce settles, so the query below only
     * ever runs against a settled cursor line, never on every keystroke of cursor movement (same
     * timer semantics as the direct-fetch effect this replaces, contract F1#17).
     */
    useEffect(() => {
        if (!editor || cursorLine === null) return
        const timer = setTimeout(() => setDebouncedBlameQuery({ projectId, path, line: cursorLine }), BLAME_DEBOUNCE_MS)
        return () => clearTimeout(timer)
    }, [editor, cursorLine, projectId, path])

    const {
        data: blameLineResult,
        isError: isBlameLineError,
        isPlaceholderData: isBlameLinePlaceholder,
    } = useQuery(gitBlameLineQueryOptions(debouncedBlameQuery ?? { projectId: null, path: null, line: null }))

    /**
     * Bridges the query's result into `blameLine` state instead of rendering `data` directly, so
     * `setBlameLine` stays externally callable the way it was before this hook owned a query —
     * `editor-pane.tsx`'s path-switch reset block calls it synchronously (render-time, before
     * `debouncedBlameQuery` has caught up to the new path) to blank the footer immediately rather than
     * showing the previous file's blame line until the next debounced fetch resolves.
     *
     * Re-runs on `path` in addition to the query result so a tab that leaves and returns to this same
     * path inside {@link BLAME_DEBOUNCE_MS} — `debouncedBlameQuery` never advancing away from it, so
     * `blameLineResult`'s reference never changes either — still un-blanks the footer the reset block
     * just cleared, instead of leaving it empty until the cursor happens to move to a different line.
     * Gated on two conditions before touching `blameLine` at all: `debouncedBlameQuery?.path === path`
     * (the debounce hasn't caught up to a path switch yet — stay blank) and `!isBlameLinePlaceholder`
     * (the query key just changed and `keepPreviousData` is still surfacing the *previous* path's
     * line — `BlameLine` carries no path of its own to tell that apart from this path's real data, so
     * showing it here would flash the old file's author/summary on the new one). While the query is
     * merely loading for *this* path with nothing to placeholder from (`blameLineResult === undefined`
     * without being an error) this deliberately leaves `blameLine` untouched, reproducing the
     * pre-query effect's behavior of leaving the last-resolved line visible until the next fetch
     * settles rather than blanking it on every cursor move. Deferred to a microtask rather than called
     * synchronously in the effect body (an effect body must not call `setState` synchronously —
     * matches the same constraint `use-editor-git-gutter-and-conflicts.ts`'s conflict-region parse
     * effect works around).
     */
    useEffect(() => {
        if (debouncedBlameQuery?.path !== path || isBlameLinePlaceholder) return
        if (isBlameLineError) {
            queueMicrotask(() => setBlameLine(null))
            return
        }
        if (blameLineResult === undefined) return
        queueMicrotask(() => setBlameLine(blameLineResult))
    }, [path, debouncedBlameQuery, isBlameLinePlaceholder, blameLineResult, isBlameLineError])

    useEffect(() => {
        const node = blameFooterTextRef.current
        if (!node) return

        const model = editor?.getModel() ?? null
        node.textContent = !blameLine || (model && blameLine.line > model.getLineCount()) ? '' : formatBlameLine(blameLine, Date.now(), currentUser)
    }, [editor, blameLine, currentUser])

    /**
     * Tracks the model's line count in state instead of reading `editor.getModel()?.getLineCount()`
     * directly during render — a render-time read of that mutable external value falls outside
     * React's reactive graph (React Compiler memoizes off `editor`/`blameOverlayEnabled` alone, with
     * no way to know the model's own line count can change independently of either), risking a stale
     * memoized value surviving a render where nothing *else* this hook reads happened to change.
     * Deps match the original pre-query effect's own re-fetch triggers (`editor`/`blameOverlayEnabled`
     * only, not on every keystroke) — see {@link gitBlameOverlayQueryOptions}'s doc comment for why
     * `lineCount` staying out of the query key needs this to only recompute on toggle, not on typing.
     * Deferred to a microtask rather than called synchronously in the effect body (an effect body must
     * not call `setState` synchronously — matches the same constraint the blame-line bridge effect
     * above works around).
     */
    useEffect(() => {
        const nextLineCount = editor && blameOverlayEnabled ? (editor.getModel()?.getLineCount() ?? null) : null
        queueMicrotask(() => setOverlayLineCount(nextLineCount))
    }, [editor, blameOverlayEnabled])

    const { data: blameOverlayLines } = useQuery(gitBlameOverlayQueryOptions({ projectId, path, lineCount: overlayLineCount }))

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

    return { setCursorLine, blameFooterTextRef, setBlameLine, setBlameOverlayEnabled }
}
