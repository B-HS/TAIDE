import { IS_MAC } from '@shared/constants/platform'
import type { AppCommand } from '@shared/lib/command-registry'
import { KEYMAP_CATEGORY } from '@shared/lib/command-registry'
import type { KeymapActionId, KeymapEvent, KeymapModifier, KeymapOverrideEntry } from '@shared/lib/keymap'
import { APP_KEYMAP, findKeymapConflict, keymapEntryToEvent, matchesKeymapEntry } from '@shared/lib/keymap'
import { MONACO_ACTIONS } from '@shared/lib/monaco-actions'
import { isMonacoCommandId, toMonacoActionId } from '@shared/lib/monaco-keybinding'

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
        isOverridden: false,
        runsViaCommand: false,
        source: 'app',
        defaultBindingLabel: null,
    }))

    return [...commandRows, ...keymapOnlyRows].map((row) => {
        const override = overrides.find((item) => item.actionId === row.id)
        return override ? { ...row, key: override.key, mods: override.mods, isOverridden: true } : row
    })
}

/**
 * A row counts as unassigned when it has no app-level key AND no effective monaco default —
 * a monaco row keeps its built-in binding until the user overrides it, so only an explicit
 * user override (rebind handled by the `key` check, unbind by `isOverridden`) disables it.
 */
export const isKeybindingRowUnassigned = (row: KeybindingRow) => !row.key && (row.isOverridden || !row.defaultBindingLabel)

export const findConflictingRow = (rows: KeybindingRow[], row: KeybindingRow, isMac: boolean = IS_MAC) =>
    row.key ? findKeymapConflict(rows, row, row.id, isMac) : null

export const filterKeybindingRowsByCapturedKey = (rows: KeybindingRow[], key: string, mods: KeymapModifier[], isMac: boolean = IS_MAC) =>
    rows.filter((row) => row.key && matchesKeymapEntry(row, keymapEntryToEvent({ key, mods }, isMac), isMac))

export const sortKeybindingRows = (rows: KeybindingRow[], getLabel: (row: KeybindingRow) => string) =>
    rows.toSorted((a, b) => {
        if (!!a.key !== !!b.key) return a.key ? -1 : 1
        return getLabel(a).localeCompare(getLabel(b))
    })

export const findRunnableCommandBinding = (rows: KeybindingRow[], event: KeymapEvent, isMac: boolean = IS_MAC) =>
    rows.find((row) => row.runsViaCommand && row.commandId && matchesKeymapEntry(row, event, isMac)) ?? null

export const findKeybindingRowById = (rows: KeybindingRow[], id: string) => rows.find((row) => row.id === id) ?? null

export const buildUnbindOverride = (rowId: string): KeymapOverrideEntry => ({ actionId: rowId, key: '', mods: [] })

export const mergeKeybindingOverride = (overrides: KeymapOverrideEntry[], next: KeymapOverrideEntry) => [
    ...overrides.filter((override) => override.actionId !== next.actionId),
    next,
]

export const removeKeybindingOverride = (overrides: KeymapOverrideEntry[], rowId: string) =>
    overrides.filter((override) => override.actionId !== rowId)
