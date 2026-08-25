import type { KeymapActionId, KeymapChordStage, KeymapEvent } from '@shared/lib/keymap/keymap'

/**
 * `entryIds` (not a single `entryId`) — several sibling `APP_KEYMAP` chord entries can share the
 * same first stage (see `findMatchingChordPrefixEntries`), so entering chord-pending must carry
 * every candidate the first-stage keydown matched, not just one arbitrarily chosen winner. Stage-2
 * resolution (`decideKeymapDispatch`'s `chordState.pending` branch) searches this list for whichever
 * entry's own second stage matches the incoming keydown.
 */
export type KeymapChordPendingState = { entryIds: KeymapActionId[]; prefix: KeymapChordStage; at: number }
export type KeymapChordStoreState = { pending: KeymapChordPendingState | null; monacoDeferral: boolean }

/** Mirrors monaco's own chord timeout (`abstractKeybindingService.js`) — see Wave H contract §2.4. */
export const KEYMAP_CHORD_PENDING_TIMEOUT_MS = 5000

type Listener = () => void

let state: KeymapChordStoreState = { pending: null, monacoDeferral: false }
const listeners = new Set<Listener>()
const noMatchListeners = new Set<Listener>()

let pendingTimeoutId: ReturnType<typeof setTimeout> | null = null
let deferralTimeoutId: ReturnType<typeof setTimeout> | null = null
let resolvePendingScheduled = false
let consumeDeferralScheduled = false

/**
 * `queueMicrotask` callbacks cannot be cancelled once scheduled — only guarded. A boolean guard
 * alone (`resolvePendingScheduled`/`consumeDeferralScheduled`) stops *re-scheduling* a duplicate
 * microtask, but a microtask already in flight when `clearKeymapChordPending`/
 * `clearKeymapChordState`/`enterKeymapChordPending` runs would still fire later and clobber
 * whatever state was written in between (e.g. resolve → clear → re-enter, all before a microtask
 * flush, clobbers the re-entered `pending`). Each mutator that can invalidate a scheduled clear
 * bumps its epoch; the deferred callback captures the epoch at schedule time and no-ops if it no
 * longer matches, i.e. if the world moved on before it got to run.
 */
let pendingEpoch = 0
let deferralEpoch = 0

const notify = () => {
    for (const listener of listeners) listener()
}

const clearPendingTimeout = () => {
    if (pendingTimeoutId === null) return
    clearTimeout(pendingTimeoutId)
    pendingTimeoutId = null
}

const clearDeferralTimeout = () => {
    if (deferralTimeoutId === null) return
    clearTimeout(deferralTimeoutId)
    deferralTimeoutId = null
}

export const getKeymapChordStoreSnapshot = () => state

let lastDispatchEvent: KeymapEvent | null = null
let lastDispatchSnapshot: KeymapChordStoreState = state

/**
 * Snapshot used specifically by `decideKeymapDispatch` callers (`use-global-keymap.ts`), keyed on
 * the physical `KeyboardEvent` reference rather than always reading the live `state`. Up to N
 * independent `useGlobalKeymap` window listeners (one per mounted consumer widget) receive the
 * *identical* event object for one physical keydown — `event.stopPropagation()` only stops
 * propagation to other DOM nodes, not sibling listeners registered on the same `window` target, so
 * every one of them calls this once per keydown. `enterKeymapChordPending`/`armKeymapMonacoDeferral`
 * mutate `state` synchronously and immediately-visibly; without memoizing by event identity, the
 * *first* listener's transition (e.g. arming the monaco-deferral window on a Cmd+K observed while
 * an editor is focused) would already be visible to the *second* listener evaluating that exact
 * same Cmd+K keydown, which would misread it as the deferral's "next keydown" and immediately
 * schedule the deferral's clear — consumed before the user's real next physical keystroke ever
 * arrives (see `docs/features/keymap.md` "멀티 리스너 팬아웃" for the traced repro). Memoizing by
 * event reference — not a timer — means every listener sees the exact pre-transition snapshot for
 * as long as they're all still looking at the one event that caused it, and the transition only
 * becomes visible starting with the next *distinct* event object, i.e. the real next keystroke.
 */
export const getKeymapChordDispatchSnapshot = (event: KeymapEvent): KeymapChordStoreState => {
    if (event !== lastDispatchEvent) {
        lastDispatchEvent = event
        lastDispatchSnapshot = state
    }
    return lastDispatchSnapshot
}

export const subscribeKeymapChordStore = (listener: Listener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}

/** Fires once whenever a stage-2 keydown was swallowed without matching the pending chord — drives the "no matching shortcut" indicator flash (`keymap.chordNoMatch`). Deliberately kept out of `KeymapChordStoreState`: it is a transient event, not a piece of state a snapshot should hold. */
export const subscribeKeymapChordNoMatch = (listener: Listener) => {
    noMatchListeners.add(listener)
    return () => {
        noMatchListeners.delete(listener)
    }
}

export const notifyKeymapChordNoMatch = () => {
    for (const listener of noMatchListeners) listener()
}

/**
 * Enters chord-pending state for a matched first stage — always for a real `APP_KEYMAP` entry
 * dispatch wait (`decideKeymapDispatch`'s `enter-chord` action). The rebind-capture UI
 * (`keybindings-editor.tsx`) records a chord being newly *defined* through its own local
 * `CaptureTarget.pendingFirstStage` state instead of this store, and is short-circuited out of the
 * dispatch path entirely while active (`use-keydown-capture.ts`'s `isCapturing` guard) — so this
 * store never needs an entry-less "recording" variant. Idempotent for equal `(entryIds, prefix)`:
 * up to 5 independent `useGlobalKeymap` window listeners (one per consumer widget) evaluate the
 * same keydown and may all call this for the same match; re-writing identical values has no
 * observable effect beyond restarting the timeout, which is itself harmless (all 5 calls land
 * within the same synchronous dispatch pass).
 */
export const enterKeymapChordPending = (
    prefix: KeymapChordStage,
    entryIds: KeymapActionId[],
    timeoutMs: number = KEYMAP_CHORD_PENDING_TIMEOUT_MS,
) => {
    pendingEpoch += 1
    const myEpoch = pendingEpoch
    clearPendingTimeout()
    state = { ...state, pending: { entryIds, prefix, at: Date.now() } }
    pendingTimeoutId = setTimeout(() => {
        pendingTimeoutId = null
        if (pendingEpoch !== myEpoch) return
        state = { ...state, pending: null }
        notify()
    }, timeoutMs)
    notify()
}

/**
 * Resolves (clears) the pending chord wait after a stage-2 keydown was consumed. Deferred to a
 * microtask rather than clearing synchronously, for the same reason as
 * {@link consumeKeymapMonacoDeferral}: up to 5 independent window `keydown` listeners (one per
 * `useGlobalKeymap` consumer widget) observe the *same* stage-2 keydown within one synchronous
 * event-dispatch pass. If the first listener cleared `pending` immediately, listeners registered
 * after it would read `pending === null` for that same event and misinterpret it as a fresh,
 * non-chord keydown instead of the chord's resolving key. Scheduling the clear as a microtask lets
 * every listener observe a stable `pending` value while resolving this one keydown — the flush
 * always happens after the synchronous dispatch for the current event completes, and always before
 * the next real event can be dispatched.
 */
export const resolveKeymapChordPending = () => {
    if (resolvePendingScheduled) return
    resolvePendingScheduled = true
    const myEpoch = pendingEpoch
    queueMicrotask(() => {
        resolvePendingScheduled = false
        if (pendingEpoch !== myEpoch) return
        clearPendingTimeout()
        if (state.pending === null) return
        state = { ...state, pending: null }
        notify()
    })
}

/** Immediate (non-deferred) clear for the app-blur/timeout exit paths, where there is no "same event, multiple listeners" race to guard against. */
export const clearKeymapChordPending = () => {
    pendingEpoch += 1
    resolvePendingScheduled = false
    clearPendingTimeout()
    if (state.pending === null) return
    state = { ...state, pending: null }
    notify()
}

/** Arms the monaco-deferral window after observing (not swallowing) monaco's own chord-prefix keydown while an editor is focused — see `MONACO_CHORD_PREFIX_KEY`. */
export const armKeymapMonacoDeferral = (timeoutMs: number = KEYMAP_CHORD_PENDING_TIMEOUT_MS) => {
    deferralEpoch += 1
    const myEpoch = deferralEpoch
    clearDeferralTimeout()
    state = { ...state, monacoDeferral: true }
    deferralTimeoutId = setTimeout(() => {
        deferralTimeoutId = null
        if (deferralEpoch !== myEpoch) return
        state = { ...state, monacoDeferral: false }
        notify()
    }, timeoutMs)
    notify()
}

/** Mirrors {@link resolveKeymapChordPending}'s microtask deferral (and its epoch guard), for the same 5-listener fan-out race. */
export const consumeKeymapMonacoDeferral = () => {
    if (consumeDeferralScheduled) return
    consumeDeferralScheduled = true
    const myEpoch = deferralEpoch
    queueMicrotask(() => {
        consumeDeferralScheduled = false
        if (deferralEpoch !== myEpoch) return
        clearDeferralTimeout()
        if (!state.monacoDeferral) return
        state = { ...state, monacoDeferral: false }
        notify()
    })
}

/**
 * Immediate clear of both `pending` and `monacoDeferral` — window blur / focus loss exit path
 * (Wave H contract §3.1). A no-op (no `notify`) when already idle: `use-keydown-capture.ts`
 * registers one `blur` listener per mounted `useGlobalKeymap` consumer, so a single OS-focus loss
 * calls this once per listener — without the guard, every one of those redundant calls would still
 * publish a new `state` object, tripping every `useSyncExternalStore` subscriber's `Object.is`
 * check (chiefly the status bar's chord indicator) for a transition that never actually happened.
 * Also drops {@link lastDispatchEvent}'s reference to the last dispatched `KeyboardEvent` — nothing
 * else ever clears it, so without this a keydown's `target` (and the DOM subtree it can pin) would
 * otherwise be held onto indefinitely whenever the app goes idle between keystrokes.
 */
export const clearKeymapChordState = () => {
    pendingEpoch += 1
    deferralEpoch += 1
    resolvePendingScheduled = false
    consumeDeferralScheduled = false
    clearPendingTimeout()
    clearDeferralTimeout()
    lastDispatchEvent = null
    if (state.pending === null && !state.monacoDeferral) return
    state = { pending: null, monacoDeferral: false }
    notify()
}
