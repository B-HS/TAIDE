import type { TabId } from '@shared/api/bindings'
import { createFireAndForgetBridge, type FireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

type WriteHandler = (data: string) => void
type QueuedWrite = { data: string; queuedAtMs: number }
type WriteSlot = { bridge: FireAndForgetBridge<string>; queue: QueuedWrite[]; lastActivityAtMs: number }

/**
 * Upper bound on how many writes a single tab's slot holds while its pty session isn't registered
 * yet. Without a cap, `requestTerminalWrite` calls for a `tabId` whose `TerminalSession` never
 * mounts (a stale tabId from a bug elsewhere, or a tab that was requested but closed before its
 * session ever spawned) would accumulate forever.
 */
export const TERMINAL_WRITE_QUEUE_MAX_ENTRIES = 500

/**
 * How long a queued write, or an entirely unregistered slot, is kept before being discarded — a
 * freshly opened terminal tab's session is expected to register within a few seconds, not sit idle
 * for minutes, so anything older than this is almost certainly not going to be delivered and is
 * only holding memory. Also drives the opportunistic slot sweep in {@link getOrCreateSlot}, which
 * closes the residual leak F0 flagged: a slot that was written to but never registered at all
 * (`hasSubscribers()` never true) would otherwise never be removed from `slotsByTabId`, since
 * nothing ever calls the unregister path that frees a *registered* slot.
 */
export const TERMINAL_WRITE_QUEUE_TTL_MS = 30_000

const slotsByTabId = new Map<TabId, WriteSlot>()

const pruneExpiredQueue = (queue: QueuedWrite[], nowMs: number, ttlMs: number) => queue.filter((entry) => nowMs - entry.queuedAtMs <= ttlMs)

const sweepStaleSlots = (nowMs: number, ttlMs: number) => {
    for (const [tabId, slot] of slotsByTabId) {
        if (slot.bridge.hasSubscribers()) continue
        if (nowMs - slot.lastActivityAtMs <= ttlMs) continue
        slotsByTabId.delete(tabId)
    }
}

const getOrCreateSlot = (tabId: TabId, nowMs: number, ttlMs: number): WriteSlot => {
    sweepStaleSlots(nowMs, ttlMs)
    const existing = slotsByTabId.get(tabId)
    if (existing) return existing
    const slot: WriteSlot = { bridge: createFireAndForgetBridge<string>({ subscriberModel: 'single-owner' }), queue: [], lastActivityAtMs: nowMs }
    slotsByTabId.set(tabId, slot)
    return slot
}

/**
 * Registers the write handler a terminal tab's live pty session accepts input through. Callers
 * (`requestTerminalWrite`) that fire before a tab's session is ready — e.g. a freshly opened
 * terminal tab still waiting on its first spawn — have their writes queued (capped at
 * {@link TERMINAL_WRITE_QUEUE_MAX_ENTRIES}, pruned by {@link TERMINAL_WRITE_QUEUE_TTL_MS}) and
 * flushed here in order, rather than silently dropped. Only one handler is ever current per tab:
 * registering a new one replaces the previous, matching a single live pty session per tab.
 */
export const registerTerminalWriteHandler = (tabId: TabId, handler: WriteHandler, ttlMs: number = TERMINAL_WRITE_QUEUE_TTL_MS) => {
    const nowMs = Date.now()
    const slot = getOrCreateSlot(tabId, nowMs, ttlMs)
    slot.lastActivityAtMs = nowMs
    for (const entry of pruneExpiredQueue(slot.queue, nowMs, ttlMs)) handler(entry.data)
    slot.queue = []

    const unsubscribe = slot.bridge.subscribe(handler)
    return () => {
        unsubscribe()
        if (!slot.bridge.hasSubscribers()) slotsByTabId.delete(tabId)
    }
}

export const requestTerminalWrite = (
    tabId: TabId,
    data: string,
    ttlMs: number = TERMINAL_WRITE_QUEUE_TTL_MS,
    maxEntries: number = TERMINAL_WRITE_QUEUE_MAX_ENTRIES,
) => {
    const nowMs = Date.now()
    const slot = getOrCreateSlot(tabId, nowMs, ttlMs)
    slot.lastActivityAtMs = nowMs

    if (slot.bridge.hasSubscribers()) {
        slot.bridge.publish(data)
        return
    }

    const fresh = pruneExpiredQueue(slot.queue, nowMs, ttlMs)
    const next = [...fresh, { data, queuedAtMs: nowMs }]
    slot.queue = next.length > maxEntries ? next.slice(next.length - maxEntries) : next
}
