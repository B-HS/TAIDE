import type { TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'

const registry = new Map<TabId, monaco.editor.IStandaloneCodeEditor>()

export const registerEditorInstance = (tabId: TabId, editor: monaco.editor.IStandaloneCodeEditor) => {
    registry.set(tabId, editor)
}

export const unregisterEditorInstance = (tabId: TabId) => {
    registry.delete(tabId)
}

export const getEditorInstance = (tabId: TabId) => registry.get(tabId) ?? null
