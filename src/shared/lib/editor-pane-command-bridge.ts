import type { TabWindowTarget } from '@shared/api/bindings'

export type TabCycleDirection = 'next' | 'prev'

export type EditorPaneCommand =
    | { type: 'split' }
    | { type: 'cycle-tab'; direction: TabCycleDirection }
    | { type: 'save-active-tab' }
    | { type: 'toggle-terminal' }
    | { type: 'run-monaco-action'; actionId: string }
    | { type: 'run-selected-text-in-terminal' }
    | { type: 'run-in-terminal'; text: string; cwd: string | null }
    | { type: 'move-focused-tab-to-window'; target: TabWindowTarget }

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
