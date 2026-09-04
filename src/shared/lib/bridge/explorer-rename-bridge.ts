import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

/**
 * "Rename" on a file tab's context menu. TAIDE has exactly one rename affordance — the explorer
 * tree's inline editor (`use-explorer-entry-crud.ts`) — so the tab menu asks the explorer to reveal
 * the path and start that editor on it, rather than growing a second rename UI with its own
 * validation and conflict rules. Broadcast, like the reveal bridge next to it: the app shell
 * listens to expand a collapsed sidebar while the explorer panel performs the reveal + rename.
 */
const renameInExplorerBridge = createFireAndForgetBridge<string>()

export const requestRenameInExplorer = renameInExplorerBridge.publish
export const subscribeRenameInExplorer = renameInExplorerBridge.subscribe
