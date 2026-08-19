import type { TabId } from '@shared/api/bindings'

type FlushFn = () => Promise<void> | void

const flushersByTabId = new Map<TabId, FlushFn>()

export const registerMirrorFlush = (tabId: TabId, flush: FlushFn) => {
    flushersByTabId.set(tabId, flush)
}

export const unregisterMirrorFlush = (tabId: TabId) => {
    flushersByTabId.delete(tabId)
}

/**
 * Same one-callback-per-tab shape as `flushersByTabId` above, kept as its own map instead of being
 * folded into it. `use-editor-file-persistence` (the hot-exit *mirror*, i.e. unsaved draft content)
 * and `use-editor-view-state` (monaco `viewState` — cursor/scroll) both register a flush keyed by
 * the exact same `tabId` for every open file tab; a single shared map would have the second hook's
 * registration silently overwrite the first's, dropping unsaved-edit recovery for every tab that
 * also has a mounted `useEditorViewState`.
 */
const viewStateFlushersByTabId = new Map<TabId, FlushFn>()

export const registerViewStateFlush = (tabId: TabId, flush: FlushFn) => {
    viewStateFlushersByTabId.set(tabId, flush)
}

export const unregisterViewStateFlush = (tabId: TabId) => {
    viewStateFlushersByTabId.delete(tabId)
}

const runFlush = async (flush: FlushFn) => {
    try {
        await flush()
    } catch {
        return undefined
    }
}

/**
 * Invokes every registered hot-exit flush — both the file-mirror ones and the viewState ones (see
 * `viewStateFlushersByTabId` above) — and waits for all of them to settle. A flush that throws
 * (synchronously or asynchronously) is swallowed so one stuck pane cannot block the rest from being
 * persisted before the app exits.
 */
export const flushAllMirrors = async () => {
    await Promise.all([...flushersByTabId.values(), ...viewStateFlushersByTabId.values()].map(runFlush))
}
