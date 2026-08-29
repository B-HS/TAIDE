import type { KeymapChordStage, KeymapModifier } from '@shared/lib/keymap/keymap'
import { normalizeKeymapKey } from '@shared/lib/keymap/keymap'

export type MonacoDefaultBinding = KeymapChordStage & { chord?: KeymapChordStage }

const MAC_MODIFIER_BY_GLYPH: Record<string, KeymapModifier> = { '⌘': 'mod', '⌥': 'alt', '⇧': 'shift', '⌃': 'ctrl' }

/**
 * Monaco's own label spellings for the non-character keys that appear in `MONACO_ACTIONS`
 * (`⌘⌫`, `⌃Space`, the F-keys). The values are the canonical keymap keys `matchesKeymapEntry`
 * compares against — `KeyboardEvent.key` for named keys, `normalizeKeymapEventKey`'s `space` for
 * the space bar (its `event.key` is a literal `' '`).
 */
const KEYMAP_KEY_BY_MONACO_LABEL: Record<string, string> = { '⌫': 'Backspace', '⌦': 'Delete', '⏎': 'Enter', '⇥': 'Tab', Space: 'space' }

const parseStage = (rawStage: string): KeymapChordStage | null => {
    const mods: KeymapModifier[] = []
    let rest = rawStage
    while (rest.length > 0 && MAC_MODIFIER_BY_GLYPH[rest[0]]) {
        mods.push(MAC_MODIFIER_BY_GLYPH[rest[0]])
        rest = rest.slice(1)
    }
    if (!rest) return null
    return { key: KEYMAP_KEY_BY_MONACO_LABEL[rest] ?? normalizeKeymapKey(rest), mods }
}

const parseLabel = (label: string): MonacoDefaultBinding | null => {
    const stages = label.split(' ')
    if (stages.length > 2) return null
    const first = parseStage(stages[0])
    if (!first) return null
    if (stages.length === 1) return first
    const second = parseStage(stages[1])
    if (!second) return null
    return { ...first, chord: second }
}

const bindingByLabel = new Map<string, MonacoDefaultBinding | null>()

/**
 * Reads a `MonacoActionEntry.defaultBindingLabel` (`'⌘D'`, `'⌘K ⌘C'`, `'⇧F10'` — mac-formatted,
 * chord stages separated by a space) back into the `{ key, mods, chord }` shape keymap matching
 * speaks, so monaco's built-in bindings can take part in conflict detection. `⌘` maps to `mod`,
 * which resolves to Ctrl off mac exactly as monaco's own `KeyMod.CtrlCmd` does; `⌃` maps to the
 * mac-only `ctrl`, matching `KeyMod.WinCtrl`.
 *
 * Display still goes through the raw label (`keybinding-row.tsx`) and monaco still owns the real
 * binding — nothing here is fed back into monaco. Results are cached per label: the catalog holds
 * a fixed handful of distinct labels, and conflict detection walks every row against every other
 * row, so returning the same parsed object keeps that pass allocation-free.
 */
export const parseMonacoDefaultBindingLabel = (label: string | null): MonacoDefaultBinding | null => {
    if (!label) return null
    const cached = bindingByLabel.get(label)
    if (cached !== undefined) return cached

    const binding = parseLabel(label)
    bindingByLabel.set(label, binding)
    return binding
}
