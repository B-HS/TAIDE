import { useEffect, useEffectEvent, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId, TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { layoutQueryOptions, useSetTabViewState } from '@entities/layout/layout.query'

type UseEditorViewStateInput = {
    projectId: ProjectId
    tabId: TabId
    editor: monaco.editor.IStandaloneCodeEditor | null
}

/**
 * Parses a `Tab.viewState`'s persisted JSON back into monaco's own shape, tolerating anything
 * malformed (a hand-edited `layout.json`, or a future monaco upgrade changing the shape) by
 * treating it the same as "nothing persisted yet" rather than throwing.
 */
export const parsePersistedViewState = (viewState: string | null | undefined) => {
    if (!viewState) return null
    try {
        return JSON.parse(viewState) as monaco.editor.ICodeEditorViewState
    } catch {
        return null
    }
}

/**
 * Persists/restores monaco's cursor+scroll `viewState` through `layout_set_view_state` (contract
 * X1#4) — independent of `entities/editor/model-registry`'s own in-memory save/restore (which
 * `CodeEditor` already does on every same-session tab switch), this hook is the only path that
 * survives an app restart: model-registry's cache is cleared with the page, `Tab.view_state` is
 * not.
 *
 * Persist fires from this effect's cleanup, not on every cursor move — cleanup for the *whole* tree
 * flushes before any effect's setup runs (React's commit phase — `EditorPane`'s
 * `registerEditorInstance` doc comment relies on the same ordering fact the other way), so this
 * cleanup still observes the *previous* tab's model when it calls `editor.saveViewState()`, before
 * `CodeEditor`'s own model-swap effect (a child, so its setup runs first in the *next* phase) ever
 * touches it. Deps are `[editor, tabId]` only — `setTabViewState`'s own identity is read through
 * `useEffectEvent` instead of listed as a dependency, so an unstable `mutate` reference from
 * `useSetTabViewState` can never itself make this cleanup+setup pair re-run and persist on every
 * render; only an actual tab switch, close, or pane unmount does. `lastSentViewStateRef` further
 * skips the IPC call (and the `revision` bump/broadcast it would cause) when the serialized value
 * hasn't actually changed since the last thing this hook knows the backend already has — the common
 * case of opening a tab and switching straight back away without moving the cursor.
 */
export const useEditorViewState = ({ projectId, tabId, editor }: UseEditorViewStateInput) => {
    const restoredTabIdsRef = useRef(new Set<TabId>())
    const lastSentViewStateRef = useRef(new Map<TabId, string>())

    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: setTabViewState } = useSetTabViewState(projectId)

    const persistViewState = useEffectEvent((tabIdToPersist: TabId, viewState: string) => {
        setTabViewState({ tabId: tabIdToPersist, viewState })
    })

    useEffect(() => {
        const lastSentViewState = lastSentViewStateRef.current
        return () => {
            if (!editor) return
            const viewState = editor.saveViewState()
            if (!viewState) return

            const serialized = JSON.stringify(viewState)
            if (lastSentViewState.get(tabId) === serialized) return

            lastSentViewState.set(tabId, serialized)
            persistViewState(tabId, serialized)
        }
    }, [editor, tabId])

    useEffect(() => {
        if (!editor || !layout || restoredTabIdsRef.current.has(tabId)) return
        restoredTabIdsRef.current.add(tabId)

        const tab = collectAllPaneTabs(layout).find((candidate) => candidate.id === tabId)
        if (tab?.viewState) lastSentViewStateRef.current.set(tabId, tab.viewState)

        const parsed = parsePersistedViewState(tab?.viewState)
        if (parsed) editor.restoreViewState(parsed)
    }, [editor, tabId, layout])
}
