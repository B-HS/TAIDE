import type { TabId } from '@shared/api/bindings'

type Listener = () => void

const contentByTabId = new Map<TabId, string>()
const listeners = new Set<Listener>()

const notify = () => {
    for (const listener of listeners) listener()
}

export const getUntitledContent = (tabId: TabId) => contentByTabId.get(tabId) ?? null

export const setUntitledContent = (tabId: TabId, content: string) => {
    contentByTabId.set(tabId, content)
    notify()
}

export const dropUntitledContent = (tabId: TabId) => {
    if (!contentByTabId.delete(tabId)) return
    notify()
}

export const pruneUntitledContents = (keepTabIds: TabId[]) => {
    const keep = new Set(keepTabIds)
    const removed = [...contentByTabId.keys()].filter((tabId) => !keep.has(tabId))
    for (const tabId of removed) contentByTabId.delete(tabId)
    if (removed.length > 0) notify()
    return removed
}

export const subscribeUntitledContent = (listener: Listener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
