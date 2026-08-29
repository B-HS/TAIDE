import type { KeybindingRow } from '@shared/lib/keymap/keybinding-catalog'
import { findRunnableCommandBinding } from '@shared/lib/keymap/keybinding-catalog'
import type { KeymapChordStage, KeymapEntry, KeymapEvent } from '@shared/lib/keymap/keymap'
import type { KeymapChordStoreState } from '@shared/lib/keymap/keymap-chord-store'
import type { KeymapContextGetters } from '@shared/lib/keymap/keymap-context'
import { decideKeymapDispatch } from '@shared/lib/keymap/keymap-dispatch'

type DecideCommandBindingRunInput = {
    rows: KeybindingRow[]
    entries: KeymapEntry[]
    event: KeymapEvent
    chordState: KeymapChordStoreState
    isMac: boolean
    getters?: KeymapContextGetters
    monacoChordPrefixes?: KeymapChordStage[]
}

/**
 * Decides whether the standalone command-binding listener (`command-palette.tsx`) may run a
 * `runsViaCommand` row for this keydown. Two independent `window` capture listeners see the same
 * physical keydown — this one and `useGlobalKeymap`'s `APP_KEYMAP` dispatcher — and
 * `stopPropagation` between siblings on the same target does nothing, so a command the user rebound
 * onto a key `APP_KEYMAP` already owns would fire *both* actions from one keypress (the conflict
 * the keybindings editor only warns about at save time, since a warning must not block the rebind).
 * Running the very same `decideKeymapDispatch` the other listener runs — it is pure, so evaluating
 * it twice for one event has no side effect — makes `APP_KEYMAP` the single winner: anything other
 * than `none` means that listener claims this keydown (a dispatch, a chord stage, or a monaco
 * yield), and the command row stands down. The `pending`/`monacoDeferral` short-circuit is kept
 * ahead of it so a mid-chord or monaco-deferred keydown never even reaches row matching.
 */
export const decideCommandBindingRun = ({ rows, entries, event, chordState, isMac, getters, monacoChordPrefixes }: DecideCommandBindingRunInput) => {
    if (chordState.pending || chordState.monacoDeferral) return null
    if (decideKeymapDispatch(event, entries, chordState, isMac, getters, monacoChordPrefixes).type !== 'none') return null
    return findRunnableCommandBinding(rows, event, isMac)
}
