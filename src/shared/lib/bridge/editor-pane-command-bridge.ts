import type { TabWindowTarget } from '@shared/api/bindings'
import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

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

const editorPaneCommandBridge = createFireAndForgetBridge<EditorPaneCommand>()

export const requestEditorPaneCommand = editorPaneCommandBridge.publish
export const subscribeEditorPaneCommand = editorPaneCommandBridge.subscribe
