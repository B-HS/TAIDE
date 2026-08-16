import { IS_MAC } from '@shared/constants/platform'

export type KeymapActionId =
    | 'quick-open'
    | 'command-palette'
    | 'workspace-symbol'
    | 'close-tab'
    | 'toggle-sidebar'
    | 'find'
    | 'search'
    | 'search-replace'
    | 'explorer'
    | 'git'
    | 'split'
    | 'tab-cycle-next'
    | 'tab-cycle-prev'
    | 'reopen-closed-tab'
    | 'save'
    | 'toggle-terminal'
    | 'new-terminal'
    | 'terminal-jump-to-previous-command'
    | 'terminal-jump-to-next-command'
    | 'font-size-up'
    | 'font-size-down'
    | 'open-keybindings-editor'

export type KeymapModifier = 'mod' | 'ctrl' | 'shift' | 'alt'

/** A single key-stage: a physical key plus the modifiers held with it. */
export type KeymapChordStage = { key: string; mods: KeymapModifier[] }

export type KeymapEntry = {
    id: KeymapActionId
    key: string
    mods: KeymapModifier[]
    /**
     * Second stage of a two-key chord (e.g. Cmd+K Cmd+S). When present, `key`/`mods` above are the
     * chord's *first* stage (the prefix) rather than a standalone binding — see
     * `findMatchingChordPrefixEntry`/`matchesChordSecondStage`. Optional so a pre-Wave-H parser
     * (which only knows `key`/`mods`) silently reads a chord entry as if it were bound to the
     * prefix alone: no throw, no data loss, just a stale single-stage interpretation until upgraded.
     */
    chord?: KeymapChordStage
    when?: string
    descriptionKey: string
}

export type KeymapEvent = {
    key: string
    code?: string
    metaKey: boolean
    ctrlKey: boolean
    shiftKey: boolean
    altKey: boolean
    /** OS key-repeat while a key is held — never counts as a fresh "next keydown" for chord/deferral consumption (see `keymap-dispatch.ts`). */
    repeat?: boolean
    /** IME composition in progress (`event.isComposing`, e.g. Korean/Japanese input) — never counts as a fresh "next keydown" either, so composing a character doesn't get eaten by a pending chord/deferral wait. */
    isComposing?: boolean
}

export const APP_KEYMAP: KeymapEntry[] = [
    { id: 'quick-open', key: 'p', mods: ['mod'], descriptionKey: 'keymap.quickOpen' },
    { id: 'command-palette', key: 'p', mods: ['mod', 'shift'], descriptionKey: 'keymap.commandPalette' },
    { id: 'workspace-symbol', key: 't', mods: ['mod'], descriptionKey: 'palette.workspaceSymbols' },
    { id: 'close-tab', key: 'w', mods: ['mod'], descriptionKey: 'keymap.closeTab' },
    { id: 'toggle-sidebar', key: 'b', mods: ['mod'], descriptionKey: 'keymap.toggleSidebar' },
    { id: 'find', key: 'f', mods: ['mod'], descriptionKey: 'keymap.find' },
    { id: 'search', key: 'f', mods: ['mod', 'shift'], descriptionKey: 'keymap.search' },
    { id: 'search-replace', key: 'h', mods: ['mod', 'shift'], descriptionKey: 'keymap.searchReplace' },
    { id: 'explorer', key: 'e', mods: ['mod', 'shift'], descriptionKey: 'keymap.explorer' },
    { id: 'git', key: 'g', mods: ['ctrl', 'shift'], descriptionKey: 'git.title' },
    { id: 'split', key: '\\', mods: ['mod'], descriptionKey: 'keymap.split' },
    { id: 'tab-cycle-next', key: 'Tab', mods: ['ctrl'], descriptionKey: 'keymap.tabCycleNext' },
    { id: 'tab-cycle-prev', key: 'Tab', mods: ['ctrl', 'shift'], descriptionKey: 'keymap.tabCyclePrev' },
    { id: 'reopen-closed-tab', key: 't', mods: ['mod', 'shift'], descriptionKey: 'keymap.reopenClosedTab' },
    { id: 'save', key: 's', mods: ['mod'], descriptionKey: 'keymap.save' },
    { id: 'toggle-terminal', key: '`', mods: ['ctrl'], descriptionKey: 'keymap.toggleTerminal' },
    { id: 'new-terminal', key: '`', mods: ['ctrl', 'shift'], descriptionKey: 'keymap.newTerminal' },
    {
        id: 'terminal-jump-to-previous-command',
        key: 'ArrowUp',
        mods: ['mod'],
        when: 'terminalFocus',
        descriptionKey: 'keymap.terminalJumpToPreviousCommand',
    },
    {
        id: 'terminal-jump-to-next-command',
        key: 'ArrowDown',
        mods: ['mod'],
        when: 'terminalFocus',
        descriptionKey: 'keymap.terminalJumpToNextCommand',
    },
    { id: 'font-size-up', key: '=', mods: ['mod'], descriptionKey: 'keymap.fontSizeUp' },
    { id: 'font-size-down', key: '-', mods: ['mod'], descriptionKey: 'keymap.fontSizeDown' },
    {
        id: 'open-keybindings-editor',
        key: 'k',
        mods: ['mod'],
        chord: { key: 's', mods: ['mod'] },
        /**
         * Terminal-scoped so the chord's first stage (Cmd/Ctrl+K) doesn't shadow the terminal's own
         * Cmd+K "clear screen" muscle memory — without this, focusing a terminal and pressing Cmd+K
         * arms this chord's pending-wait (swallowing the very next keystroke, matched or not) instead
         * of ever reaching xterm. `terminalFocus` is already a whitelisted context getter
         * (`keymap-context.ts`); `findMatchingChordPrefixEntry` consults `when` for every candidate,
         * `when`-less entries included, so this is the one entry that opts *out* of that default.
         */
        when: '!terminalFocus',
        descriptionKey: 'settings.keymapOpenEditor',
    },
]

/**
 * Monaco's own default chord-prefix keybinding (Cmd+K on mac, Ctrl+K elsewhere) — the first stage
 * of all ~21 built-in monaco chords (Cmd+K Cmd+B, Cmd+K Cmd+S, ...). Tracked independently of
 * whatever chord-carrying entries `APP_KEYMAP` happens to define: the monaco-deferral window
 * (`armKeymapMonacoDeferral`) must protect monaco's chord namespace even when the app defines no
 * chord of its own under this prefix, and must *not* expand to protect prefixes the app *does* own
 * (an app chord's own first stage already gets its `!editorTextFocus` gate — see
 * `findMatchingChordPrefixEntry`). See `docs/acknowledge/2026-08-16-wave-h-keymap-contract.md` §2.2-3.
 */
export const MONACO_CHORD_PREFIX_KEY: KeymapChordStage = { key: 'k', mods: ['mod'] }

/**
 * A stored entry.key may come from either of two capture schemes: the current physical-key
 * scheme (`normalizeKeymapEventKey` — code-based, survives macOS Option composition) or the
 * legacy scheme it replaced (raw `event.key`, lowercased). Entries saved under the legacy
 * scheme have no `code` on record to re-derive the physical key from, so matching accepts
 * either derivation of the live event rather than requiring a one-time data migration that
 * would be lossy for composed/dead keys (Option+K, Option+Space, ...).
 */
const matchesEntryKey = (entryKey: string, event: KeymapEvent) => {
    const canonicalEntryKey = canonicalizeKeymapKey(entryKey).toLowerCase()
    const physicalKey = normalizeKeymapEventKey(event).toLowerCase()
    const legacyKey = canonicalizeKeymapKey(normalizeKeymapKey(event.key)).toLowerCase()
    return canonicalEntryKey === physicalKey || canonicalEntryKey === legacyKey
}

export const matchesKeymapEntry = (entry: Pick<KeymapEntry, 'key' | 'mods'>, event: KeymapEvent, isMac: boolean) => {
    if (!entry.key) return false
    if (!matchesEntryKey(entry.key, event)) return false

    const wantsMeta = entry.mods.includes('mod') && isMac
    const wantsCtrl = entry.mods.includes('ctrl') || (entry.mods.includes('mod') && !isMac)
    const wantsShift = entry.mods.includes('shift')
    const wantsAlt = entry.mods.includes('alt')

    return event.metaKey === wantsMeta && event.ctrlKey === wantsCtrl && event.shiftKey === wantsShift && event.altKey === wantsAlt
}

/** Always-satisfied default for the `isWhenSatisfied` parameter below — callers with no context evaluator get today's unconditional matching. */
const alwaysSatisfiedWhen = () => true

/**
 * Matches single-stage entries only (no `chord`) — a chord-carrying entry's `key`/`mods` is its
 * *first* stage, which must go through `findMatchingChordPrefixEntry` instead so pressing the
 * prefix alone never fires the chord's action. `isWhenSatisfied` is injected (not evaluated here)
 * so this module stays DOM/monaco-free and unit-testable without a browser context; real callers
 * pass an evaluator built from `keymap-context.ts` getters + `keymap-when.ts`.
 *
 * An entry with no `when` always matches *without* consulting `isWhenSatisfied` at all — this is a
 * structural guarantee, not merely a convention every caller must remember, that the 19 `when`-less
 * `APP_KEYMAP` entries stay behaviorally unchanged no matter what evaluator a caller passes.
 */
export const findMatchingKeymapEntry = (
    entries: KeymapEntry[],
    event: KeymapEvent,
    isMac: boolean = IS_MAC,
    isWhenSatisfied: (when: string | undefined) => boolean = alwaysSatisfiedWhen,
) =>
    entries.find((entry) => !entry.chord && matchesKeymapEntry(entry, event, isMac) && (entry.when === undefined || isWhenSatisfied(entry.when))) ??
    null

/**
 * Matches a chord entry's *first* stage (the prefix) — entries without `chord` are never
 * candidates. Unlike {@link findMatchingKeymapEntry}, `isWhenSatisfied` is consulted for *every*
 * candidate, including ones with no `when` of their own: chord dispatch's implicit
 * `!editorTextFocus` gate (Wave H contract §3.1) lives in the caller-supplied predicate itself
 * (see `keymap-dispatch.ts`), not in each entry's `when` field, so it must never be bypassed.
 */
export const findMatchingChordPrefixEntry = (
    entries: KeymapEntry[],
    event: KeymapEvent,
    isMac: boolean = IS_MAC,
    isWhenSatisfied: (when: string | undefined) => boolean = alwaysSatisfiedWhen,
) => entries.find((entry) => entry.chord && matchesKeymapEntry(entry, event, isMac) && isWhenSatisfied(entry.when)) ?? null

/** Matches a chord entry's *second* stage while the store's `pending` state names `entry` as the one awaiting completion. No `when` gate — reaching stage 2 already implies the app committed to owning this chord at stage 1. */
export const matchesChordSecondStage = (entry: Pick<KeymapEntry, 'chord'>, event: KeymapEvent, isMac: boolean = IS_MAC) =>
    entry.chord !== undefined && matchesKeymapEntry(entry.chord, event, isMac)

export type KeymapOverrideEntry = {
    actionId: string
    key: string
    mods: KeymapModifier[]
    /** Rebinding an entry is a complete restatement of its binding — an absent `chord` here means the user confirmed a single-key bind, not "keep the base entry's chord" (see `applyKeymapOverrides`). */
    chord?: KeymapChordStage
}

const isKeymapOverrideEntry = (value: unknown): value is KeymapOverrideEntry =>
    typeof value === 'object' && value !== null && 'actionId' in value && 'key' in value && 'mods' in value

/**
 * Same permissiveness level as {@link isKeymapOverrideEntry}'s own `key`/`mods` check (shape-only,
 * not a full `KeymapModifier[]` validation) — a malformed `chord` from hand-edited settings JSON
 * must not reach `formatKeymapStage`'s `stage.mods.includes(...)`, which throws on a non-array.
 */
const isValidKeymapChordStage = (value: unknown): value is KeymapChordStage =>
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { key?: unknown }).key === 'string' &&
    Array.isArray((value as { mods?: unknown }).mods)

/**
 * Legacy `actionId`s from before an `AppCommand` gained a `keymapId` pointing at a renamed
 * `APP_KEYMAP` entry — without this, a stored override for the old id silently stops matching any
 * row (`buildKeybindingRows`/`applyKeymapOverrides` both key off `KeymapOverrideEntry.actionId`)
 * and the user's rebind is lost with no error. `keybindings.open` was ungated (no `keymapId`) before
 * Wave H introduced `open-keybindings-editor` as its dedicated `APP_KEYMAP` chord entry.
 */
const LEGACY_KEYMAP_OVERRIDE_ACTION_ID_ALIASES: Record<string, string> = {
    'keybindings.open': 'open-keybindings-editor',
}

/** Drops a malformed `chord` rather than the whole entry (its 1st stage `key`/`mods` stay intact — mirrors the pre-existing "an unparseable extra field never destroys the base binding" stance {@link isKeymapOverrideEntry} already takes for forward-compat). Migrates a legacy `actionId` in the same pass. */
const sanitizeKeymapOverrideEntry = (entry: KeymapOverrideEntry): KeymapOverrideEntry => {
    const actionId = LEGACY_KEYMAP_OVERRIDE_ACTION_ID_ALIASES[entry.actionId] ?? entry.actionId
    const chord = entry.chord !== undefined && !isValidKeymapChordStage(entry.chord) ? undefined : entry.chord
    return actionId === entry.actionId && chord === entry.chord ? entry : { ...entry, actionId, chord }
}

export const parseKeymapOverrides = (json: string | null): KeymapOverrideEntry[] => {
    if (!json) return []

    let parsed: unknown
    try {
        parsed = JSON.parse(json)
    } catch {
        return []
    }

    return Array.isArray(parsed) ? parsed.filter(isKeymapOverrideEntry).map(sanitizeKeymapOverrideEntry) : []
}

export const serializeKeymapOverrides = (overrides: KeymapOverrideEntry[]) => JSON.stringify(overrides)

export const applyKeymapOverrides = (baseEntries: KeymapEntry[], overrides: KeymapOverrideEntry[]): KeymapEntry[] =>
    baseEntries.map((entry) => {
        const override = overrides.find((item) => item.actionId === entry.id)
        return override ? { ...entry, key: override.key, mods: override.mods, chord: override.chord } : entry
    })

export const keymapEntryToEvent = (entry: Pick<KeymapEntry, 'key' | 'mods'>, isMac: boolean = IS_MAC): KeymapEvent => ({
    key: entry.key,
    metaKey: entry.mods.includes('mod') && isMac,
    ctrlKey: entry.mods.includes('ctrl') || (entry.mods.includes('mod') && !isMac),
    shiftKey: entry.mods.includes('shift'),
    altKey: entry.mods.includes('alt'),
})

/** Two `when`-scoped entries can share a key combo without conflicting only when *both* declare a scope and it differs — either side left unscoped (`when` absent) still risks a real overlap. */
const hasDisjointKeymapWhenScopes = (a: string | undefined, b: string | undefined) => a !== undefined && b !== undefined && a !== b

const areKeymapModsEqual = (a: KeymapModifier[], b: KeymapModifier[]) => a.length === b.length && a.every((mod) => b.includes(mod))

/**
 * Two chord-carrying entries sharing the same first-stage key+mods can share it without
 * conflicting only when *both* declare a second stage and it differs — VS Code's own ⌘K
 * namespace holds ~21 sibling chords this way. A chord entry vs. a plain (non-chord) entry that
 * shares the same first-stage key+mods always conflicts regardless of this check: the chord's
 * `findMatchingChordPrefixEntry` match wins dispatch priority ahead of plain matching
 * (`decideKeymapDispatch`), permanently shadowing the plain entry, so leaving either side
 * chord-less must NOT excuse the collision (mirrors `hasDisjointKeymapWhenScopes`'s stance on an
 * unscoped side).
 */
const hasDisjointKeymapChordStages = (a: KeymapChordStage | undefined, b: KeymapChordStage | undefined) =>
    a !== undefined && b !== undefined && (a.key.toLowerCase() !== b.key.toLowerCase() || !areKeymapModsEqual(a.mods, b.mods))

export const findKeymapConflict = <T extends { id: string; key: string; mods: KeymapModifier[]; when?: string; chord?: KeymapChordStage }>(
    entries: T[],
    candidate: Pick<KeymapEntry, 'key' | 'mods' | 'when' | 'chord'>,
    excludeId: string,
    isMac: boolean = IS_MAC,
) =>
    entries.find(
        (entry) =>
            entry.id !== excludeId &&
            !hasDisjointKeymapWhenScopes(entry.when, candidate.when) &&
            !hasDisjointKeymapChordStages(entry.chord, candidate.chord) &&
            matchesKeymapEntry(entry, keymapEntryToEvent(candidate, isMac), isMac),
    ) ?? null

const MAC_MODIFIER_ORDER: KeymapModifier[] = ['ctrl', 'alt', 'shift', 'mod']
const NON_MAC_MODIFIER_ORDER: KeymapModifier[] = ['mod', 'ctrl', 'alt', 'shift']
const MAC_MODIFIER_LABEL: Record<KeymapModifier, string> = { mod: '⌘', ctrl: '⌃', alt: '⌥', shift: '⇧' }
const NON_MAC_MODIFIER_LABEL: Record<KeymapModifier, string> = { mod: 'Ctrl', ctrl: 'Ctrl', alt: 'Alt', shift: 'Shift' }

const formatKeymapStage = (stage: KeymapChordStage, isMac: boolean) => {
    const modifierLabel = isMac ? MAC_MODIFIER_LABEL : NON_MAC_MODIFIER_LABEL
    const modifierOrder = isMac ? MAC_MODIFIER_ORDER : NON_MAC_MODIFIER_ORDER
    const labels = modifierOrder.filter((mod) => stage.mods.includes(mod)).map((mod) => modifierLabel[mod])
    const dedupedLabels = labels.filter((label, index) => labels.indexOf(label) === index)
    return [...dedupedLabels, stage.key.toUpperCase()].join(isMac ? '' : '+')
}

/** A chord's second stage is joined with a space after the first, mirroring monaco's own `defaultBindingLabel` chord notation ("⌘K ⌘S" / "Ctrl+K Ctrl+S"). */
export const formatKeymapShortcut = (entry: Pick<KeymapEntry, 'key' | 'mods' | 'chord'>, isMac: boolean = IS_MAC) => {
    const firstStage = formatKeymapStage(entry, isMac)
    if (!entry.chord) return firstStage
    return [firstStage, formatKeymapStage(entry.chord, isMac)].join(' ')
}

export const captureModsFromEvent = (
    event: Pick<KeymapEvent, 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey'>,
    isMac: boolean = IS_MAC,
): KeymapModifier[] => {
    const mods: KeymapModifier[] = []
    if (isMac && event.metaKey) mods.push('mod')
    if (isMac && event.ctrlKey) mods.push('ctrl')
    if (!isMac && event.ctrlKey) mods.push('mod')
    if (event.shiftKey) mods.push('shift')
    if (event.altKey) mods.push('alt')
    return mods
}

export const normalizeKeymapKey = (key: string) => (key.length === 1 ? key.toLowerCase() : key)

const canonicalizeKeymapKey = (key: string) => (key === ' ' ? 'space' : key)

const CLEAN_SINGLE_KEY_PATTERN = /^[a-z0-9\-=[\];'`,./\\ ]$/i
const LETTER_EVENT_CODE_PATTERN = /^Key([A-Z])$/
const DIGIT_EVENT_CODE_PATTERN = /^Digit([0-9])$/
const PUNCTUATION_KEY_BY_EVENT_CODE: Record<string, string> = {
    Space: 'space',
    Minus: '-',
    Equal: '=',
    BracketLeft: '[',
    BracketRight: ']',
    Semicolon: ';',
    Quote: "'",
    Backquote: '`',
    Comma: ',',
    Period: '.',
    Slash: '/',
    Backslash: '\\',
}

/**
 * Derives the canonical keymap key from a keyboard event. A clean bindable single character in
 * `event.key` wins (keeps non-US layouts correct — monaco resolves through the layout-aware legacy
 * keyCode). When `event.key` arrives composed or dead instead (macOS Option combos: Option+Space
 * = U+00A0, Option+K = '˚', Option+E = 'Dead'), the physical `event.code` supplies the canonical
 * key for the layout-stable set (Space, KeyA-Z, Digit0-9, basic punctuation). Everything else
 * (Enter, Tab, arrows, F-keys, ...) keeps the `event.key` path.
 */
export const normalizeKeymapEventKey = (event: Pick<KeymapEvent, 'key' | 'code'>) => {
    if (CLEAN_SINGLE_KEY_PATTERN.test(event.key)) return canonicalizeKeymapKey(event.key.toLowerCase())
    const code = event.code ?? ''
    const letterMatch = LETTER_EVENT_CODE_PATTERN.exec(code)
    if (letterMatch) return letterMatch[1].toLowerCase()
    const digitMatch = DIGIT_EVENT_CODE_PATTERN.exec(code)
    if (digitMatch) return digitMatch[1]
    return PUNCTUATION_KEY_BY_EVENT_CODE[code] ?? normalizeKeymapKey(event.key)
}

/**
 * `event.key` values that never count as "the next real keydown" for chord stage-2 resolution or
 * the monaco-deferral window — a bare modifier press (or a toggle/layout key the user doesn't
 * perceive as "typing a key") arriving mid-wait must not consume it. Covers both the W3C UI Events
 * `KeyboardEvent.key` modifier values and the lock/layout keys (`CapsLock`, `NumLock`,
 * `ScrollLock`, `AltGraph`, the Windows/Linux `OS` key some browsers still report, and `Fn`).
 */
export const MODIFIER_ONLY_KEYS = ['Shift', 'Control', 'Alt', 'Meta', 'AltGraph', 'CapsLock', 'NumLock', 'ScrollLock', 'OS', 'Fn']
