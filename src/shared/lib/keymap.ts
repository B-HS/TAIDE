import { IS_MAC } from '@shared/constants/platform'

export type KeymapActionId =
    | 'quick-open'
    | 'command-palette'
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
    | 'font-size-up'
    | 'font-size-down'

export type KeymapModifier = 'mod' | 'ctrl' | 'shift' | 'alt'

export type KeymapEntry = {
    id: KeymapActionId
    key: string
    mods: KeymapModifier[]
    when?: string
    descriptionKey: string
}

export type KeymapEvent = {
    key: string
    metaKey: boolean
    ctrlKey: boolean
    shiftKey: boolean
    altKey: boolean
}

export const APP_KEYMAP: KeymapEntry[] = [
    { id: 'quick-open', key: 'p', mods: ['mod'], descriptionKey: 'keymap.quickOpen' },
    { id: 'command-palette', key: 'p', mods: ['mod', 'shift'], descriptionKey: 'keymap.commandPalette' },
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
    { id: 'font-size-up', key: '=', mods: ['mod'], descriptionKey: 'keymap.fontSizeUp' },
    { id: 'font-size-down', key: '-', mods: ['mod'], descriptionKey: 'keymap.fontSizeDown' },
]

export const matchesKeymapEntry = (entry: KeymapEntry, event: KeymapEvent, isMac: boolean) => {
    if (event.key.toLowerCase() !== entry.key.toLowerCase()) return false

    const wantsMeta = entry.mods.includes('mod') && isMac
    const wantsCtrl = entry.mods.includes('ctrl') || (entry.mods.includes('mod') && !isMac)
    const wantsShift = entry.mods.includes('shift')
    const wantsAlt = entry.mods.includes('alt')

    return event.metaKey === wantsMeta && event.ctrlKey === wantsCtrl && event.shiftKey === wantsShift && event.altKey === wantsAlt
}

export const findMatchingKeymapEntry = (entries: KeymapEntry[], event: KeymapEvent, isMac: boolean = IS_MAC) =>
    entries.find((entry) => matchesKeymapEntry(entry, event, isMac)) ?? null

export type KeymapOverrideEntry = {
    actionId: KeymapActionId
    key: string
    mods: KeymapModifier[]
}

const isKeymapOverrideEntry = (value: unknown): value is KeymapOverrideEntry =>
    typeof value === 'object' && value !== null && 'actionId' in value && 'key' in value && 'mods' in value

export const parseKeymapOverrides = (json: string | null): KeymapOverrideEntry[] => {
    if (!json) return []

    let parsed: unknown
    try {
        parsed = JSON.parse(json)
    } catch {
        return []
    }

    return Array.isArray(parsed) ? parsed.filter(isKeymapOverrideEntry) : []
}

export const serializeKeymapOverrides = (overrides: KeymapOverrideEntry[]) => JSON.stringify(overrides)

export const applyKeymapOverrides = (baseEntries: KeymapEntry[], overrides: KeymapOverrideEntry[]): KeymapEntry[] =>
    baseEntries.map((entry) => {
        const override = overrides.find((item) => item.actionId === entry.id)
        return override ? { ...entry, key: override.key, mods: override.mods } : entry
    })

export const keymapEntryToEvent = (entry: Pick<KeymapEntry, 'key' | 'mods'>, isMac: boolean = IS_MAC): KeymapEvent => ({
    key: entry.key,
    metaKey: entry.mods.includes('mod') && isMac,
    ctrlKey: entry.mods.includes('ctrl') || (entry.mods.includes('mod') && !isMac),
    shiftKey: entry.mods.includes('shift'),
    altKey: entry.mods.includes('alt'),
})

export const findKeymapConflict = (
    entries: KeymapEntry[],
    candidate: Pick<KeymapEntry, 'key' | 'mods'>,
    excludeActionId: KeymapActionId,
    isMac: boolean = IS_MAC,
) => entries.find((entry) => entry.id !== excludeActionId && matchesKeymapEntry(entry, keymapEntryToEvent(candidate, isMac), isMac)) ?? null

export const formatKeymapShortcut = (entry: Pick<KeymapEntry, 'key' | 'mods'>, isMac: boolean = IS_MAC) => {
    const modLabel = entry.mods.includes('mod') ? (isMac ? '⌘' : 'Ctrl') : ''
    const otherLabels = entry.mods.filter((mod) => mod !== 'mod').map((mod) => (mod === 'shift' ? '⇧' : mod === 'alt' ? '⌥' : 'Ctrl'))
    return [...otherLabels, modLabel, entry.key.toUpperCase()].filter(Boolean).join('')
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
