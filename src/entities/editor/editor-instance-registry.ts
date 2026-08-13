import type { TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'

type Listener = () => void

const registry = new Map<TabId, monaco.editor.IStandaloneCodeEditor>()
const listenersByTabId = new Map<TabId, Set<Listener>>()

const notifyTabListeners = (tabId: TabId) => {
    for (const listener of listenersByTabId.get(tabId) ?? []) listener()
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
