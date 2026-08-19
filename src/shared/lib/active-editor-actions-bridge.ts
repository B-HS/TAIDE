import { createExternalStoreBridge } from '@shared/lib/external-store-bridge'

const activeEditorActionIdsStore = createExternalStoreBridge<Set<string> | null>(null)

/**
 * Published by `editor-area` whenever the focused pane's active editor (or its model) changes.
 * Holds the A-tier `getSupportedActions()` id set for that editor, used to gray out monaco
 * commands that are not currently supported (command palette / keybindings editor).
 */
export const setActiveEditorActionIds = activeEditorActionIdsStore.setValue
export const subscribeActiveEditorActionIds = activeEditorActionIdsStore.subscribe
export const getActiveEditorActionIdsSnapshot = activeEditorActionIdsStore.getSnapshot
