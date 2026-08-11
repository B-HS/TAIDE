export type TabCycleDirection = 'next' | 'prev'

export type EditorPaneCommand =
    { type: 'split' } | { type: 'cycle-tab'; direction: TabCycleDirection } | { type: 'save-active-tab' } | { type: 'toggle-terminal' }

type EditorPaneCommandListener = (command: EditorPaneCommand) => void

const listeners = new Set<EditorPaneCommandListener>()

export const requestEditorPaneCommand = (command: EditorPaneCommand) => {
    for (const listener of listeners) listener(command)
}

export const subscribeEditorPaneCommand = (listener: EditorPaneCommandListener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
