import { IS_MAC } from '@shared/constants/platform'
import type { AppCommand } from '@shared/lib/command-registry'
import type { KeymapActionId, KeymapChordStage, KeymapEvent, KeymapModifier, KeymapOverrideEntry } from '@shared/lib/keymap/keymap'
import { APP_KEYMAP, findKeymapConflict, keymapEntryToEvent, matchesKeymapEntry } from '@shared/lib/keymap/keymap'
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

export const findConflictingRow = (rows: KeybindingRow[], row: KeybindingRow, isMac: boolean = IS_MAC) => {
    const candidate = resolveKeybindingRowBinding(row)
    if (!candidate.key) return null
    return findKeymapConflict(rows, { key: candidate.key, mods: candidate.mods, chord: candidate.chord }, row.id, isMac, resolveKeybindingRowBinding)
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
