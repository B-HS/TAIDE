import { IS_MAC } from '@shared/constants/platform'

export type KeymapActionId =
    | 'quick-open'
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
    description: string
}

export type KeymapEvent = {
    key: string
    metaKey: boolean
    ctrlKey: boolean
    shiftKey: boolean
    altKey: boolean
}

export const APP_KEYMAP: KeymapEntry[] = [
    { id: 'quick-open', key: 'p', mods: ['mod'], description: '파일 퀵 오픈' },
    { id: 'close-tab', key: 'w', mods: ['mod'], description: '탭 닫기' },
    { id: 'toggle-sidebar', key: 'b', mods: ['mod'], description: '사이드바 토글' },
    { id: 'search', key: 'f', mods: ['mod', 'shift'], description: '검색' },
    { id: 'explorer', key: 'e', mods: ['mod', 'shift'], description: '탐색기' },
    { id: 'git', key: 'g', mods: ['ctrl', 'shift'], description: 'Git' },
    { id: 'split', key: '\\', mods: ['mod'], description: '에디터 분할' },
    { id: 'tab-cycle-next', key: 'Tab', mods: ['ctrl'], description: '다음 탭' },
    { id: 'tab-cycle-prev', key: 'Tab', mods: ['ctrl', 'shift'], description: '이전 탭' },
    { id: 'reopen-closed-tab', key: 't', mods: ['mod', 'shift'], description: '닫은 탭 다시 열기' },
    { id: 'save', key: 's', mods: ['mod'], description: '저장' },
    { id: 'toggle-terminal', key: '`', mods: ['ctrl'], description: '터미널 토글' },
    { id: 'new-terminal', key: '`', mods: ['ctrl', 'shift'], description: '새 터미널' },
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
