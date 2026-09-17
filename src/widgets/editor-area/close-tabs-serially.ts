import type { TabId } from '@shared/api/bindings'
import { isNotFoundIpcError } from '@shared/lib/ipc-error-message'

/**
 * Closes `tabIds` one after another — every close answers with a fresh layout the next one would
 * otherwise have to be computed against — and reports what actually went wrong, rather than
 * abandoning the rest of the list at the first rejection.
 *
 * The four tab-bar menu loops and ⌘K ⌘W all walk a render-time snapshot of the strip, so a tab can
 * disappear underneath them between two iterations: Claude Code's `close_tab` IDE tool, the
 * explorer's delete follow-up (`close_file_tabs_under`), another OS window. `layout_close_tab` then
 * answers `NotFound`, which is the outcome the caller wanted anyway — it is skipped silently and the
 * walk continues. Anything else is collected so the caller can say once that the command only partly
 * ran; every other action in the tab bar toasts on failure, and staying silent here taught the user
 * that "no toast" means "everything closed".
 */
export const closeTabsSerially = async (tabIds: readonly TabId[], closeTab: (tabId: TabId) => Promise<unknown>) => {
    const failures: unknown[] = []
    for (const tabId of tabIds) {
        try {
            await closeTab(tabId)
        } catch (error) {
            if (!isNotFoundIpcError(error)) failures.push(error)
        }
    }
    return failures
}
