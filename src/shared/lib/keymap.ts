import { IS_MAC } from '@shared/constants/platform'

export type KeymapActionId =
    | 'quick-open'
    | 'command-palette'
    | 'close-tab'
    | 'toggle-sidebar'
    | 'search'
    | 'explorer'
    | 'git'
    | 'split'
    | 'tab-cycle-next'
    | 'tab-cycle-prev'
    | 'reopen-closed-tab'
    | 'save'
    | 'toggle-terminal'
    | 'new-terminal'

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
    { id: 'search', key: 'f', mods: ['mod', 'shift'], descriptionKey: 'keymap.search' },
    { id: 'explorer', key: 'e', mods: ['mod', 'shift'], descriptionKey: 'keymap.explorer' },
    { id: 'git', key: 'g', mods: ['ctrl', 'shift'], descriptionKey: 'git.title' },
    { id: 'split', key: '\\', mods: ['mod'], descriptionKey: 'keymap.split' },
    { id: 'tab-cycle-next', key: 'Tab', mods: ['ctrl'], descriptionKey: 'keymap.tabCycleNext' },
    { id: 'tab-cycle-prev', key: 'Tab', mods: ['ctrl', 'shift'], descriptionKey: 'keymap.tabCyclePrev' },
    { id: 'reopen-closed-tab', key: 't', mods: ['mod', 'shift'], descriptionKey: 'keymap.reopenClosedTab' },
    { id: 'save', key: 's', mods: ['mod'], descriptionKey: 'keymap.save' },
    { id: 'toggle-terminal', key: '`', mods: ['ctrl'], descriptionKey: 'keymap.toggleTerminal' },
    { id: 'new-terminal', key: '`', mods: ['ctrl', 'shift'], descriptionKey: 'keymap.newTerminal' },
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
