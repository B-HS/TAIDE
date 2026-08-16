import type { KeymapActionId, KeymapChordStage, KeymapEntry, KeymapEvent } from '@shared/lib/keymap'
import {
    MODIFIER_ONLY_KEYS,
    MONACO_CHORD_PREFIX_KEY,
    findMatchingChordPrefixEntry,
    findMatchingKeymapEntry,
    matchesChordSecondStage,
    matchesKeymapEntry,
} from '@shared/lib/keymap'
import type { KeymapChordStoreState } from '@shared/lib/keymap-chord-store'
import type { KeymapContextGetters } from '@shared/lib/keymap-context'
import { DEFAULT_KEYMAP_CONTEXT_GETTERS, getKeymapContextValue } from '@shared/lib/keymap-context'
import { evaluateKeymapWhen } from '@shared/lib/keymap-when'

/**
 * Pure "what should happen for this keydown" decision, kept free of DOM/store side effects so the
 * whole chord/when state machine is unit-testable without a browser or React. The caller
 * (`use-global-keymap.ts`) is the only place that actually calls `preventDefault`, mutates
 * `keymap-chord-store.ts`, or invokes a handler — see that hook for exactly which action types get
 * which side effects.
 */
export type KeymapDispatchAction =
    | { type: 'ignore-modifier-only' }
    | { type: 'defer-to-monaco' }
    | { type: 'observe-monaco-chord-prefix' }
    | { type: 'enter-chord'; entryId: KeymapActionId; prefix: KeymapChordStage }
    | { type: 'resolve-chord-matched'; entryId: KeymapActionId }
    | { type: 'resolve-chord-no-match' }
    | { type: 'dispatch'; entryId: KeymapActionId }
    | { type: 'none' }

const isModifierOnlyKey = (key: string) => MODIFIER_ONLY_KEYS.includes(key)

/**
 * `true` for a keydown that never counts as "the next real keydown" for chord stage-2 resolution
 * or the monaco-deferral window — a bare modifier press ({@link isModifierOnlyKey}), an OS
 * key-repeat while a key is held (`event.repeat`), or an IME composition step (`event.isComposing`,
 * Korean/Japanese/etc. input). None of these represent a user "typing a key" the wait should
 * consume: a repeat would otherwise resolve the wait against the *same* physical keypress that
 * armed it, and a composition keydown would otherwise eat the first character of composed text.
 */
const isIgnorableKeydown = (event: KeymapEvent) => isModifierOnlyKey(event.key) || event.repeat === true || event.isComposing === true

/**
 * Decides the single next state transition for one keydown, given the current chord-store
 * snapshot. Branch order (mirrors Wave H contract §3.1, "실행 구조"):
 *
 * 1. Monaco-deferral armed → this keydown is monaco's, not ours (except an ignorable keydown —
 *    see {@link isIgnorableKeydown} — which never counts as "the next keydown" for either this or
 *    chord stage-2 resolution).
 * 2. Chord pending → this keydown resolves stage 2 (swallowed unconditionally, matched or not).
 * 3. A chord entry's stage 1 matches, and — since chord entries carry an implicit
 *    `!editorTextFocus` gate regardless of their own `when` (Wave H contract §2.5/§3.1) — the
 *    editor isn't focused → enter chord-pending.
 * 4. Otherwise, while the editor *is* focused, monaco's own chord-prefix key (Cmd/Ctrl+K) — or any
 *    additional prefix a `monaco.*` override's own `chord` field introduces via
 *    `monacoChordPrefixes` — is merely observed (never swallowed) to arm the monaco-deferral window
 *    for the *next* keydown, independent of whether `entries` defines any *app* chord under that
 *    same prefix (see `MONACO_CHORD_PREFIX_KEY`).
 * 5. Normal single-stage matching, `when`-gated, exactly as before chords/when existed.
 */
export const decideKeymapDispatch = (
    event: KeymapEvent,
    entries: KeymapEntry[],
    chordState: KeymapChordStoreState,
    isMac: boolean,
    getters: KeymapContextGetters = DEFAULT_KEYMAP_CONTEXT_GETTERS,
    monacoChordPrefixes: KeymapChordStage[] = [MONACO_CHORD_PREFIX_KEY],
): KeymapDispatchAction => {
    if (chordState.monacoDeferral) {
        if (isIgnorableKeydown(event)) return { type: 'ignore-modifier-only' }
        return { type: 'defer-to-monaco' }
    }

    if (chordState.pending) {
        if (isIgnorableKeydown(event)) return { type: 'ignore-modifier-only' }
        const pendingEntry = entries.find((entry) => entry.id === chordState.pending?.entryId) ?? null
        if (pendingEntry && matchesChordSecondStage(pendingEntry, event, isMac)) return { type: 'resolve-chord-matched', entryId: pendingEntry.id }
        return { type: 'resolve-chord-no-match' }
    }

    const isEditorTextFocused = getKeymapContextValue('editorTextFocus', getters)
    const isWhenSatisfied = (when: string | undefined) => !isEditorTextFocused && evaluateKeymapWhen(when, getters)
    const chordEntry = findMatchingChordPrefixEntry(entries, event, isMac, isWhenSatisfied)
    if (chordEntry) return { type: 'enter-chord', entryId: chordEntry.id, prefix: { key: chordEntry.key, mods: chordEntry.mods } }

    if (isEditorTextFocused && monacoChordPrefixes.some((prefix) => matchesKeymapEntry(prefix, event, isMac))) {
        return { type: 'observe-monaco-chord-prefix' }
    }

    const entry = findMatchingKeymapEntry(entries, event, isMac, (when) => evaluateKeymapWhen(when, getters))
    if (!entry) return { type: 'none' }
    return { type: 'dispatch', entryId: entry.id }
}
