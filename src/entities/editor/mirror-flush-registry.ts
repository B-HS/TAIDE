import type { TabId } from '@shared/api/bindings'

type FlushFn = () => Promise<void> | void

const flushersByTabId = new Map<TabId, FlushFn>()

export const registerMirrorFlush = (tabId: TabId, flush: FlushFn) => {
    flushersByTabId.set(tabId, flush)
}

export const unregisterMirrorFlush = (tabId: TabId) => {
    flushersByTabId.delete(tabId)
}

const runFlush = async (flush: FlushFn) => {
    try {
        await flush()
    } catch {
        return undefined
    }
}

/**
 * Invokes every registered hot-exit mirror flush (one per currently mounted editor pane) and
 * waits for all of them to settle. A flush that throws (synchronously or asynchronously) is
 * swallowed so one stuck pane cannot block the rest from being persisted before the app exits.
 */
export const flushAllMirrors = async () => {
    await Promise.all([...flushersByTabId.values()].map(runFlush))
}
