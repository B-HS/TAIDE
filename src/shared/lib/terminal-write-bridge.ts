import type { TabId } from '@shared/api/bindings'

type WriteHandler = (data: string) => void

const handlersByTabId = new Map<TabId, WriteHandler>()
const pendingWritesByTabId = new Map<TabId, string[]>()

/**
 * Registers the write handler a terminal tab's live pty session accepts input through. Callers
 * (`requestTerminalWrite`) that fire before a tab's session is ready — e.g. a freshly opened
 * terminal tab still waiting on its first spawn — have their writes queued and flushed here in
 * order, rather than silently dropped.
 */
export const registerTerminalWriteHandler = (tabId: TabId, handler: WriteHandler) => {
    handlersByTabId.set(tabId, handler)

    const queued = pendingWritesByTabId.get(tabId)
    if (queued) {
        pendingWritesByTabId.delete(tabId)
        for (const data of queued) handler(data)
    }

    return () => {
        if (handlersByTabId.get(tabId) === handler) handlersByTabId.delete(tabId)
    }
}

export const requestTerminalWrite = (tabId: TabId, data: string) => {
    const handler = handlersByTabId.get(tabId)
    if (handler) {
        handler(data)
        return
    }

    const queued = pendingWritesByTabId.get(tabId) ?? []
    queued.push(data)
    pendingWritesByTabId.set(tabId, queued)
}
