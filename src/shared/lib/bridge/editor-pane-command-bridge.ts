import type { TabWindowTarget } from '@shared/api/bindings'
import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'
import type { PaneDirection } from '@shared/lib/pane-tree'

export type TabCycleDirection = 'next' | 'prev'

/**
 * Which editor group a `focus-group` command means — a neighbour of the focused one (⌘K ⌘←/→/↑/↓)
 * or an absolute 1-based position in leaf order (⌘1..⌘9). One command type with two targets rather
 * than two commands: both resolve to a `PaneId` through `pane-tree.ts` and then take the exact same
 * focus path, so the split belongs in the payload, not in the dispatch table.
 */
export type PaneFocusTarget = { kind: 'direction'; direction: PaneDirection } | { kind: 'position'; position: number }

export type EditorPaneCommand =
    | { type: 'split' }
    | { type: 'cycle-tab'; direction: TabCycleDirection }
    | { type: 'save-active-tab' }
    | { type: 'toggle-terminal' }
    | { type: 'run-monaco-action'; actionId: string }
    | { type: 'run-selected-text-in-terminal' }
    | { type: 'run-in-terminal'; text: string; cwd: string | null }
    | { type: 'move-focused-tab-to-window'; target: TabWindowTarget }
    | { type: 'focus-group'; target: PaneFocusTarget }
    | { type: 'move-tab-to-group'; direction: PaneDirection }
    | { type: 'close-all-tabs' }

const editorPaneCommandBridge = createFireAndForgetBridge<EditorPaneCommand>()

export const requestEditorPaneCommand = editorPaneCommandBridge.publish
export const subscribeEditorPaneCommand = editorPaneCommandBridge.subscribe
