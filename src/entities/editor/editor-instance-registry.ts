import type { TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'

type Listener = () => void

const registry = new Map<TabId, monaco.editor.IStandaloneCodeEditor>()
const listenersByTabId = new Map<TabId, Set<Listener>>()

/**
 * Calls each subscriber for `tabId` inside its own try/catch (crash-class-seal-contract.md §4,
 * boundary-6): `registerEditorInstance`/`unregisterEditorInstance` are invoked synchronously from
 * inside `CodeEditor`'s own React effect, so without this a throwing subscriber (e.g. a status-bar
 * or breadcrumbs `attachToEditor` callback with its own bug) would propagate out through the
 * registry call and get caught by whichever `ErrorBoundary` happens to wrap the CALLER — mislabeling
 * an unrelated area's bug as an editor-area crash and leaving the actually-buggy subscriber
 * mounted. Isolating each listener here keeps one subscriber's failure from taking down another
 * region's boundary attribution.
 */
const notifyTabListeners = (tabId: TabId) => {
    for (const listener of listenersByTabId.get(tabId) ?? []) {
        try {
            listener()
        } catch (error) {
            console.error(error)
        }
    }
}

export const registerEditorInstance = (tabId: TabId, editor: monaco.editor.IStandaloneCodeEditor) => {
    registry.set(tabId, editor)
    notifyTabListeners(tabId)
}

export const unregisterEditorInstance = (tabId: TabId) => {
    registry.delete(tabId)
    notifyTabListeners(tabId)
}

export const getEditorInstance = (tabId: TabId) => registry.get(tabId) ?? null

/**
 * Notifies `listener` whenever the editor instance registered for `tabId` changes
 * (mount or unmount). Consumers that read `getEditorInstance` at effect-setup time can miss
 * a later mount (the editor pane may still be loading its file query) — subscribing closes that gap.
 */
export const subscribeEditorInstance = (tabId: TabId, listener: Listener) => {
    const listeners = listenersByTabId.get(tabId) ?? new Set()
    listeners.add(listener)
    listenersByTabId.set(tabId, listeners)
    return () => {
        listeners.delete(listener)
        if (listeners.size === 0) listenersByTabId.delete(tabId)
    }
}
