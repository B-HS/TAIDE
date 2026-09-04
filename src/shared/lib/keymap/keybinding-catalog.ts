import { IS_MAC } from '@shared/constants/platform'
import type { AppCommand } from '@shared/lib/command-registry'
import type { KeymapActionId, KeymapChordStage, KeymapEntry, KeymapEvent, KeymapModifier, KeymapOverrideEntry } from '@shared/lib/keymap/keymap'
import { APP_KEYMAP, findKeymapConflict, keymapEntryToEvent, matchesKeymapEntry, normalizeKeymapEventKey } from '@shared/lib/keymap/keymap'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap/keymap-category'
import { MONACO_ACTIONS } from '@shared/lib/monaco/monaco-actions'
import { parseMonacoDefaultBindingLabel } from '@shared/lib/monaco/monaco-binding-label'
import { isMonacoCommandId, toMonacoActionId } from '@shared/lib/monaco/monaco-keybinding'

export type KeybindingRowSource = 'app' | 'monaco'

export type KeybindingRow = {
    id: string
    titleKey: string
    titleDefaultValue: string | null
    categoryKey: string | null
    commandId: string | null
    keymapId: KeymapActionId | null
    key: string
    mods: KeymapModifier[]
    chord?: KeymapChordStage
    isOverridden: boolean
    runsViaCommand: boolean
    source: KeybindingRowSource
    defaultBindingLabel: string | null
}

const MONACO_DEFAULT_BINDING_LABEL = new Map(MONACO_ACTIONS.map((entry) => [entry.actionId, entry.defaultBindingLabel]))

const KEYMAP_ONLY_CATEGORY: Partial<Record<KeymapActionId, string>> = {
    'command-palette': KEYMAP_CATEGORY.APP,
    'font-size-up': KEYMAP_CATEGORY.EDITOR,
    'font-size-down': KEYMAP_CATEGORY.EDITOR,
    'terminal-jump-to-previous-command': KEYMAP_CATEGORY.TERMINAL,
    'terminal-jump-to-next-command': KEYMAP_CATEGORY.TERMINAL,
}

export const buildKeybindingRows = (commands: AppCommand[], overrides: KeymapOverrideEntry[]): KeybindingRow[] => {
    const commandKeymapIds = new Set(commands.map((command) => command.keymapId).filter((id): id is KeymapActionId => !!id))

    const commandRows = commands.map((command): KeybindingRow => {
        const baseEntry = command.keymapId ? (APP_KEYMAP.find((entry) => entry.id === command.keymapId) ?? null) : null
        const isMonaco = isMonacoCommandId(command.id)
        return {
            id: command.keymapId ?? command.id,
            titleKey: command.titleKey,
            titleDefaultValue: command.titleDefaultValue ?? null,
            categoryKey: command.categoryKey ?? null,
            commandId: command.id,
            keymapId: command.keymapId ?? null,
            key: baseEntry?.key ?? '',
            mods: baseEntry?.mods ?? [],
            chord: baseEntry?.chord,
            isOverridden: false,
            runsViaCommand: !command.keymapId && !isMonaco,
            source: isMonaco ? 'monaco' : 'app',
            defaultBindingLabel: isMonaco ? (MONACO_DEFAULT_BINDING_LABEL.get(toMonacoActionId(command.id)) ?? null) : null,
        }
    })

    const keymapOnlyRows = APP_KEYMAP.filter((entry) => !commandKeymapIds.has(entry.id)).map((entry): KeybindingRow => ({
        id: entry.id,
        titleKey: entry.descriptionKey,
        titleDefaultValue: null,
        categoryKey: KEYMAP_ONLY_CATEGORY[entry.id] ?? null,
        commandId: null,
        keymapId: entry.id,
        key: entry.key,
        mods: entry.mods,
        chord: entry.chord,
        isOverridden: false,
        runsViaCommand: false,
        source: 'app',
        defaultBindingLabel: null,
    }))

    return [...commandRows, ...keymapOnlyRows].map((row) => {
        const override = overrides.find((item) => item.actionId === row.id)
        return override ? { ...row, key: override.key, mods: override.mods, chord: override.chord, isOverridden: true } : row
    })
}

/**
 * A row counts as unassigned when it has no app-level key AND no effective monaco default —
 * a monaco row keeps its built-in binding until the user overrides it, so only an explicit
 * user override (rebind handled by the `key` check, unbind by `isOverridden`) disables it.
 */
export const isKeybindingRowUnassigned = (row: KeybindingRow) => !row.key && (row.isOverridden || !row.defaultBindingLabel)

/**
 * The binding a row actually answers to. A monaco row carries `key: ''` until the user rebinds it
 * — its live binding is monaco's own built-in, which the catalog only knows as the display label
 * — so conflict detection used to treat all ~200 of them as unbound and stayed silent while a user
 * rebound an app action straight onto ⌘D, ⌘/, F12, ... The parsed label restores them to the
 * comparison. An explicit override wins (that is the row's real binding now), and an unbind
 * (`isOverridden` with an empty key) genuinely means "nothing", so neither consults the label.
 */
export const resolveKeybindingRowBinding = (row: KeybindingRow) =>
    row.key || row.isOverridden ? row : (parseMonacoDefaultBindingLabel(row.defaultBindingLabel) ?? row)

/**
 * The part of `findKeymapConflict`'s verdict that is pure equality, reduced to one string so rows
 * that could possibly collide can be bucketed instead of compared pairwise.
 *
 * `matchesKeymapEntry(binding, keymapEntryToEvent(candidate, isMac), isMac)` is exactly "the two
 * bindings agree on the canonical key AND on all four modifier booleans", because a synthesized
 * candidate event carries no `code`: `matchesEntryKey`'s two accepted derivations of the event key
 * (`normalizeKeymapEventKey`'s physical key and the legacy `normalizeKeymapKey(event.key)`) collapse
 * to the same string without one, and both `matchesKeymapEntry`'s `wants*` and `keymapEntryToEvent`
 * derive their booleans from `mods`/`isMac` with the identical formulas. The four booleans are a
 * fixed-arity tail, so no key — space-bearing or not — can make two different bindings render the
 * same string.
 *
 * Everything that is *not* pure equality — `when` scoping, chord-stage disjointness, the exclusion
 * of the row itself, and first-in-array-order tie-breaking — deliberately stays inside
 * {@link findKeymapConflict}, which {@link findConflictingRowInIndex} still runs over the bucket.
 * The signature only ever narrows the candidate set; it never decides a conflict.
 */
const toKeybindingSignature = (binding: Pick<KeymapEntry, 'key' | 'mods'>, isMac: boolean) => {
    const event = keymapEntryToEvent(binding, isMac)
    const canonicalKey = normalizeKeymapEventKey({ key: binding.key }).toLowerCase()
    return `${canonicalKey} ${event.metaKey} ${event.ctrlKey} ${event.shiftKey} ${event.altKey}`
}

export type KeybindingConflictIndex = { rowsByBinding: Map<string, KeybindingRow[]>; isMac: boolean }

/**
 * Buckets every bound row by {@link toKeybindingSignature} so conflict lookups stop being a scan of
 * the whole catalog. The keybindings editor asks for a conflict once per row to count them, once
 * more per row to render its warning, and again per row while the "conflicts only" filter is on —
 * O(n) scans per row over ~200 rows, re-run on every keystroke in the search box (contract
 * `2026-09-04-usability-batch4-contract.md` §C.2-4 L2). One index built per render turns each of
 * those into a map lookup plus a walk of the handful of rows that share the exact binding.
 *
 * Rows with no effective binding are left out: `matchesKeymapEntry` rejects an empty `key` outright,
 * so they can never be anyone's conflict. Insertion order is preserved inside each bucket, which is
 * what keeps `findKeymapConflict`'s "first match in array order" answer identical to a full scan's.
 */
export const buildKeybindingConflictIndex = (rows: KeybindingRow[], isMac: boolean = IS_MAC): KeybindingConflictIndex => {
    const rowsByBinding = new Map<string, KeybindingRow[]>()

    for (const row of rows) {
        const binding = resolveKeybindingRowBinding(row)
        if (!binding.key) continue
        const signature = toKeybindingSignature(binding, isMac)
        const bucket = rowsByBinding.get(signature)
        if (bucket) bucket.push(row)
        else rowsByBinding.set(signature, [row])
    }

    return { rowsByBinding, isMac }
}

/**
 * The row `row` collides with, or `null`. `row` need not be one of the indexed rows — the editor
 * asks this about a binding the user is *about to* assign, using an index built from the catalog as
 * it stands. Callers with a one-off question build a throwaway index for it; there is deliberately
 * no second "just scan the array" entry point, so the conflict rule has exactly one implementation.
 */
export const findConflictingRowInIndex = (index: KeybindingConflictIndex, row: KeybindingRow) => {
    const candidate = resolveKeybindingRowBinding(row)
    if (!candidate.key) return null

    const bucket = index.rowsByBinding.get(toKeybindingSignature(candidate, index.isMac))
    if (!bucket) return null

    return findKeymapConflict(
        bucket,
        { key: candidate.key, mods: candidate.mods, chord: candidate.chord },
        row.id,
        index.isMac,
        resolveKeybindingRowBinding,
    )
}

export const filterKeybindingRowsByCapturedKey = (rows: KeybindingRow[], key: string, mods: KeymapModifier[], isMac: boolean = IS_MAC) =>
    rows.filter((row) => row.key && matchesKeymapEntry(row, keymapEntryToEvent({ key, mods }, isMac), isMac))

export const sortKeybindingRows = (rows: KeybindingRow[], getLabel: (row: KeybindingRow) => string) =>
    rows.toSorted((a, b) => {
        if (!!a.key !== !!b.key) return a.key ? -1 : 1
        return getLabel(a).localeCompare(getLabel(b))
    })

/**
 * Only matches a row's *first* stage and only when it has no `chord` — this dispatch path
 * (`command-palette.tsx`'s standalone command-binding listener) doesn't participate in the
 * chord/monaco-deferral state machine `decideKeymapDispatch` drives for `APP_KEYMAP`/`monaco.*`
 * rows, so a `runsViaCommand` row the user rebinds to a chord would otherwise fire on its first
 * stage alone (immediately, on a single keypress) while every catalog/palette surface still labels
 * it as a two-stage shortcut — a display/behavior mismatch. Treating a chord-carrying row as
 * unbound *through this path* (rather than trying to half-implement two-stage dispatch here) is
 * the same stance `findMatchingKeymapEntry` takes for `APP_KEYMAP` chord entries.
 */
export const findRunnableCommandBinding = (rows: KeybindingRow[], event: KeymapEvent, isMac: boolean = IS_MAC) =>
    rows.find((row) => row.runsViaCommand && row.commandId && !row.chord && matchesKeymapEntry(row, event, isMac)) ?? null

export const findKeybindingRowById = (rows: KeybindingRow[], id: string) => rows.find((row) => row.id === id) ?? null

export const buildUnbindOverride = (rowId: string): KeymapOverrideEntry => ({ actionId: rowId, key: '', mods: [] })

export const mergeKeybindingOverride = (overrides: KeymapOverrideEntry[], next: KeymapOverrideEntry) => [
    ...overrides.filter((override) => override.actionId !== next.actionId),
    next,
]

export const removeKeybindingOverride = (overrides: KeymapOverrideEntry[], rowId: string) =>
    overrides.filter((override) => override.actionId !== rowId)
