export type FireAndForgetBridgeEmptyPolicy = 'ignore' | 'queue-latest' | 'queue-all'
export type FireAndForgetBridgeSubscriberModel = 'broadcast' | 'single-owner'

export type FireAndForgetBridgeOptions = {
    emptyPolicy?: FireAndForgetBridgeEmptyPolicy
    subscriberModel?: FireAndForgetBridgeSubscriberModel
}

export type FireAndForgetBridge<Payload> = {
    publish: (payload: Payload) => void
    subscribe: (listener: (payload: Payload) => void) => () => void
    hasSubscribers: () => boolean
}

const EMPTY_POLICY_DEFAULT: FireAndForgetBridgeEmptyPolicy = 'ignore'
const SUBSCRIBER_MODEL_DEFAULT: FireAndForgetBridgeSubscriberModel = 'broadcast'

/**
 * Creates a request/subscribe event channel — the shared shape behind TAIDE's cross-widget
 * "bridge" modules (`*-bridge.ts`), which let one part of the UI request an action owned by
 * another part (open a dialog, reveal a file, toggle a panel) without holding a reference to it.
 *
 * `subscriberModel` picks the delivery topology:
 * - `'broadcast'` (default): every currently subscribed listener receives each published payload.
 * - `'single-owner'`: at most one listener is "current" at a time. Subscribing replaces the
 *   previous owner outright (matching a `Map.set` overwrite, not a `Set` add), and an owner's
 *   own unsubscribe is a no-op once it has already been replaced by a newer subscribe — safe
 *   against out-of-order effect cleanup racing a remount for the same key.
 *
 * `emptyPolicy` picks what a `publish` call does while nothing is subscribed:
 * - `'ignore'` (default): the payload is dropped.
 * - `'queue-latest'`: the payload replaces any previously queued one. The next subscriber
 *   receives only the most recently published payload, once.
 * - `'queue-all'`: payloads accumulate in publish order. The next subscriber receives the full
 *   backlog, in order, once — the queue is then cleared.
 *
 * `hasSubscribers()` lets a caller decide *before* publishing whether anything would receive it
 * (e.g. to report "unhandled" back to a caller that expects a boolean, such as monaco's
 * `openCodeEditor`) — publish itself never returns a value.
 */
export const createFireAndForgetBridge = <Payload = undefined>(options: FireAndForgetBridgeOptions = {}): FireAndForgetBridge<Payload> => {
    const emptyPolicy = options.emptyPolicy ?? EMPTY_POLICY_DEFAULT
    const subscriberModel = options.subscriberModel ?? SUBSCRIBER_MODEL_DEFAULT

    type Listener = (payload: Payload) => void

    const listeners = new Set<Listener>()
    let owner: Listener | null = null
    let queuedLatest: { payload: Payload } | null = null
    let queuedAll: Payload[] = []

    const hasSubscribers = () => (subscriberModel === 'single-owner' ? owner !== null : listeners.size > 0)

    const deliver = (payload: Payload) => {
        if (subscriberModel === 'single-owner') {
            owner?.(payload)
            return
        }
        for (const listener of listeners) listener(payload)
    }

    const publish = (payload: Payload) => {
        if (hasSubscribers()) {
            deliver(payload)
            return
        }
        if (emptyPolicy === 'queue-latest') queuedLatest = { payload }
        if (emptyPolicy === 'queue-all') queuedAll = [...queuedAll, payload]
    }

    const flushTo = (listener: Listener) => {
        if (queuedLatest) {
            const { payload } = queuedLatest
            queuedLatest = null
            listener(payload)
            return
        }
        if (queuedAll.length === 0) return
        const backlog = queuedAll
        queuedAll = []
        for (const payload of backlog) listener(payload)
    }

    const subscribe = (listener: Listener) => {
        if (subscriberModel === 'single-owner') {
            owner = listener
            flushTo(listener)
            return () => {
                if (owner === listener) owner = null
            }
        }
        listeners.add(listener)
        flushTo(listener)
        return () => {
            listeners.delete(listener)
        }
    }

    return { publish, subscribe, hasSubscribers }
}
