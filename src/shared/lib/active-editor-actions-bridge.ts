type Listener = () => void

const listeners = new Set<Listener>()

let activeEditorActionIds: Set<string> | null = null

/**
 * Published by `editor-area` whenever the focused pane's active editor (or its model) changes.
 * Holds the A-tier `getSupportedActions()` id set for that editor, used to gray out monaco
 * commands that are not currently supported (command palette / keybindings editor).
 */
export const setActiveEditorActionIds = (ids: Set<string> | null) => {
    activeEditorActionIds = ids
    for (const listener of listeners) listener()
}

export const subscribeActiveEditorActionIds = (listener: Listener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}

export const getActiveEditorActionIdsSnapshot = () => activeEditorActionIds
