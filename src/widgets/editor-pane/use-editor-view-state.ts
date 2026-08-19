import { useEffect, useEffectEvent, useLayoutEffect, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId, TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { registerViewStateFlush, unregisterViewStateFlush } from '@entities/editor/mirror-flush-registry'
import { layoutQueryOptions, useSetTabViewState } from '@entities/layout/layout.query'

type UseEditorViewStateInput = {
    projectId: ProjectId
    tabId: TabId
    editor: monaco.editor.IStandaloneCodeEditor | null
}

type ViewStateReadableEditor = Pick<monaco.editor.IStandaloneCodeEditor, 'saveViewState'>
type ModelAttachedEditor = Pick<monaco.editor.IStandaloneCodeEditor, 'getModel'>

const isPlainObject = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)

/**
 * Parses a `Tab.viewState`'s persisted JSON back into monaco's own shape, tolerating anything
 * malformed (a hand-edited `layout.json`, or a future monaco upgrade changing the shape) by
 * treating it the same as "nothing persisted yet" rather than throwing — both for outright invalid
 * JSON and for JSON that parses fine but isn't shaped like a viewState (`layout_set_view_state`
 * accepts any string, and is reachable from a remote peer, so this boundary can't assume the value
 * it receives is one this hook itself ever wrote). Only checked at the top level (`cursorState` is
 * an array, `viewState` is an object) — deliberately not validating field-by-field, so a genuine
 * future monaco upgrade that adds/renames nested fields still passes through to monaco's own
 * `restoreViewState`, which already guards its field reads defensively.
 */
export const parsePersistedViewState = (viewState: string | null | undefined) => {
    if (!viewState) return null
    try {
        const parsed: unknown = JSON.parse(viewState)
        if (!isPlainObject(parsed)) return null
        if (!Array.isArray(parsed.cursorState)) return null
        if (!isPlainObject(parsed.viewState)) return null
        return parsed as unknown as monaco.editor.ICodeEditorViewState
    } catch {
        return null
    }
}

/**
 * Reads `editor`'s current viewState and serializes it, collapsing every "nothing worth persisting"
 * case into a single `null` instead of making each call site re-derive the same checks: no live
 * editor, a disposed instance (monaco's `saveViewState()` itself returns `null` once `_modelData` is
 * cleared — the same internal guard `restoreViewState`/`getModel` apply, see `codeEditorWidget.js`),
 * or a value unchanged from the last one this hook is known to have sent for `targetTabId`
 * (`lastSent` — the common case of opening a tab and switching straight back away without moving the
 * cursor, which would otherwise cost a wasted `layout_set_view_state` round trip and revision bump).
 */
export const captureChangedViewState = (editor: ViewStateReadableEditor | null, targetTabId: TabId, lastSent: Map<TabId, string>) => {
    if (!editor) return null
    const viewState = editor.saveViewState()
    if (!viewState) return null

    const serialized = JSON.stringify(viewState)
    if (lastSent.get(targetTabId) === serialized) return null
    return serialized
}

/**
 * A `restoreViewState` call only actually reaches monaco's model when the editor instance passed in
 * is still attached to one. `EditorPane` has no `key`, so switching to a tab this session hasn't
 * cached yet removes `CodeEditor` from the tree for one commit (`isPending` — see `EditorPane`)
 * while this hook's restore effect still re-runs with the *previous* commit's `editor`: an instance
 * `CodeEditor`'s own unmount cleanup has already `dispose()`d by the time this (passive) effect
 * fires, since a passive effect's own cleanup always follows every layout effect *and* every deleted
 * subtree's synchronous teardown in React's commit order. A disposed editor's
 * `restoreViewState`/`getModel` both short-circuit on monaco's internal `_modelData` being cleared,
 * so the call silently no-ops rather than throwing — this check is what lets the caller tell that
 * apart from a real restore, so it doesn't spend `restoredTabIdsRef`'s once-per-instance budget on a
 * call that never touched a model.
 */
export const hasRestorableModel = (editor: ModelAttachedEditor) => editor.getModel() !== null

/**
 * Persists/restores monaco's cursor+scroll `viewState` through `layout_set_view_state` (contract
 * X1#4) — independent of `entities/editor/model-registry`'s own in-memory save/restore (which
 * `CodeEditor` already does on every same-session model swap), this hook is the only path that
 * survives an app restart: model-registry's cache is cleared with the page, `Tab.view_state` is not.
 *
 * Saving happens on two occasions, both funneled through `captureChangedViewState`'s shared dedup
 * (`lastSentViewStateRef`) so persisting the same value twice is a no-op rather than a duplicate IPC
 * call:
 *
 *  1. **This hook's own `[editor, tabId]` transitions** — a tab switch within this pane (`EditorPane`
 *     has no `key`, so switching tabs re-renders this hook with a new `tabId` while `editor` state is
 *     still the *previous* tab's instance for this commit) or this whole `EditorPane` unmounting (tab
 *     closed, pane removed). Deliberately a `useLayoutEffect`, not a passive `useEffect` (the original
 *     design) — React runs every *layout* effect's cleanup, for the whole commit, synchronously
 *     during the commit's mutation/layout phase, strictly before any *passive* effect cleanup at all,
 *     including a deleted child's. `CodeEditor`'s `editor.dispose()` lives in a passive effect's
 *     cleanup, so when the new tab is a cache miss and `CodeEditor` unmounts *in the same commit* as
 *     the `tabId` change, a passive save effect here would already see a disposed instance
 *     (`saveViewState()` returning `null` — the original bug: cursor/scroll silently never persisted
 *     for the everyday "open a file via the explorer/quick-open" path). A layout effect's cleanup
 *     fires before that dispose ever runs, whether `CodeEditor` stays mounted (same-instance model
 *     swap) or is removed this commit, so `editor` is always still live here.
 *  2. **The app exits / the window closes** — neither of the above fires (no re-render, no unmount:
 *     the process is torn down out from under the page), so this registers with the same hot-exit
 *     flush registry `use-editor-file-persistence` uses for its own mirror —
 *     `registerViewStateFlush`, a *second* map in that registry, not the one that hook uses, since
 *     both hooks key their registration by this identical `tabId` and a shared map would have one
 *     overwrite the other.
 *
 * Restoring is capped at once per `tabId` per `EditorPane` instance (`restoredTabIdsRef`) — but only
 * once a restore attempt actually reaches a live model (`hasRestorableModel`); see that function's
 * doc comment for why a naive "attempted once" gate burns the budget on a no-op. It reads the
 * `LAYOUT.DETAIL` query with a `select` narrowed to just this `tabId`'s `viewState` string rather
 * than subscribing to the whole `ProjectLayout` — every layout mutation (tab activation, the dirty
 * flag, even this hook's own persisted writes) otherwise re-rendered every open `EditorPane`, for a
 * value the restore effect below only ever reads once per tab anyway.
 *
 * Restoring here always wins over `entities/editor/model-registry`'s own in-session restore for the
 * same `path` (`CodeEditor`'s model-swap effect, a child, sets up before this hook's restore effect,
 * a parent, gets to run — see that effect's own placement note). That only matters the one time a
 * file is opened in a *second* pane in the same session: the persisted (possibly older) viewState
 * wins over the first pane's current position, rather than the two staying independent. Harmless in
 * the common case (a tab's restore effect only ever runs meaningfully once), so this is documented
 * as accepted behavior rather than reconciled — model-registry is a different layer's cache and
 * reconciling the two would mean this hook reaching into it.
 */
export const useEditorViewState = ({ projectId, tabId, editor }: UseEditorViewStateInput) => {
    const restoredTabIdsRef = useRef(new Set<TabId>())
    const lastSentViewStateRef = useRef(new Map<TabId, string>())

    const { data: persistedViewState } = useQuery({
        ...layoutQueryOptions(projectId),
        select: (layout) => collectAllPaneTabs(layout).find((candidate) => candidate.id === tabId)?.viewState ?? null,
    })
    const { mutateAsync: setTabViewStateAsync } = useSetTabViewState(projectId)

    const persistViewState = useEffectEvent(async (tabIdToPersist: TabId, serialized: string) => {
        lastSentViewStateRef.current.set(tabIdToPersist, serialized)
        await setTabViewStateAsync({ tabId: tabIdToPersist, viewState: serialized }).catch(() => undefined)
    })

    useLayoutEffect(() => {
        const lastSentViewState = lastSentViewStateRef.current
        return () => {
            const serialized = captureChangedViewState(editor, tabId, lastSentViewState)
            if (serialized) void persistViewState(tabId, serialized)
        }
    }, [editor, tabId])

    useEffect(() => {
        const flush = async () => {
            const serialized = captureChangedViewState(editor, tabId, lastSentViewStateRef.current)
            if (serialized) await persistViewState(tabId, serialized)
        }
        registerViewStateFlush(tabId, flush)
        return () => unregisterViewStateFlush(tabId)
    }, [editor, tabId])

    useEffect(() => {
        if (!editor || persistedViewState === undefined || restoredTabIdsRef.current.has(tabId)) return
        if (!hasRestorableModel(editor)) return

        restoredTabIdsRef.current.add(tabId)

        if (persistedViewState) lastSentViewStateRef.current.set(tabId, persistedViewState)

        const parsed = parsePersistedViewState(persistedViewState)
        if (parsed) editor.restoreViewState(parsed)
    }, [editor, tabId, persistedViewState])
}
