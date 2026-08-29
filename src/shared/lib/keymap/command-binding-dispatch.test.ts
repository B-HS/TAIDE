import { describe, expect, test } from 'bun:test'
import type { KeybindingRow } from '@shared/lib/keymap/keybinding-catalog'
import { decideCommandBindingRun } from '@shared/lib/keymap/command-binding-dispatch'
import type { KeymapEntry, KeymapEvent } from '@shared/lib/keymap/keymap'
import type { KeymapChordStoreState } from '@shared/lib/keymap/keymap-chord-store'
import type { KeymapContextGetters } from '@shared/lib/keymap/keymap-context'

const noContext: KeymapContextGetters = {}
const idleChordState: KeymapChordStoreState = { pending: null, monacoDeferral: false }

const toggleSidebarEntry: KeymapEntry = { id: 'toggle-sidebar', key: 'b', mods: ['mod'], descriptionKey: 'keymap.toggleSidebar' }
const chordEntry: KeymapEntry = {
    id: 'open-keybindings-editor',
    key: 'k',
    mods: ['mod'],
    chord: { key: 's', mods: ['mod'] },
    descriptionKey: 'settings.keymapOpenEditor',
}

const commandRow = (key: string): KeybindingRow => ({
    id: 'settings.open',
    titleKey: 'settings.title',
    titleDefaultValue: null,
    categoryKey: null,
    commandId: 'settings.open',
    keymapId: null,
    key,
    mods: ['mod'],
    isOverridden: true,
    runsViaCommand: true,
    source: 'app',
    defaultBindingLabel: null,
})

const keyEvent = (key: string): KeymapEvent => ({ key, metaKey: true, ctrlKey: false, shiftKey: false, altKey: false })

describe('decideCommandBindingRun', () => {
    test('APP 키맵이 쓰지 않는 키로 재바인딩한 커맨드는 실행 후보가 된다', () => {
        const row = decideCommandBindingRun({
            rows: [commandRow('j')],
            entries: [toggleSidebarEntry],
            event: keyEvent('j'),
            chordState: idleChordState,
            isMac: true,
            getters: noContext,
        })

        expect(row?.commandId).toBe('settings.open')
    })

    test('APP 키맵 엔트리와 같은 키로 재바인딩하면 커맨드 쪽은 실행하지 않는다 (이중 발동 차단)', () => {
        const row = decideCommandBindingRun({
            rows: [commandRow('b')],
            entries: [toggleSidebarEntry],
            event: keyEvent('b'),
            chordState: idleChordState,
            isMac: true,
            getters: noContext,
        })

        expect(row).toBeNull()
    })

    test('chord 1단과 같은 키면 chord 진입이 우선이라 커맨드는 실행하지 않는다', () => {
        const row = decideCommandBindingRun({
            rows: [commandRow('k')],
            entries: [chordEntry],
            event: keyEvent('k'),
            chordState: idleChordState,
            isMac: true,
            getters: noContext,
        })

        expect(row).toBeNull()
    })

    test('chord 2단 대기 중에는 어떤 커맨드도 실행하지 않는다', () => {
        const row = decideCommandBindingRun({
            rows: [commandRow('j')],
            entries: [toggleSidebarEntry],
            event: keyEvent('j'),
            chordState: { pending: { entryIds: ['open-keybindings-editor'], prefix: { key: 'k', mods: ['mod'] }, at: 0 }, monacoDeferral: false },
            isMac: true,
            getters: noContext,
        })

        expect(row).toBeNull()
    })

    test('monaco 양보 창이 열려 있으면 커맨드를 실행하지 않는다', () => {
        const row = decideCommandBindingRun({
            rows: [commandRow('j')],
            entries: [toggleSidebarEntry],
            event: keyEvent('j'),
            chordState: { pending: null, monacoDeferral: true },
            isMac: true,
            getters: noContext,
        })

        expect(row).toBeNull()
    })

    test('when 이 충족되지 않아 APP 키맵이 놓치는 키는 커맨드가 그대로 가져간다', () => {
        const terminalScopedEntry: KeymapEntry = {
            id: 'terminal-jump-to-previous-command',
            key: 'b',
            mods: ['mod'],
            when: 'terminalFocus',
            descriptionKey: 'keymap.terminalJumpToPreviousCommand',
        }

        const row = decideCommandBindingRun({
            rows: [commandRow('b')],
            entries: [terminalScopedEntry],
            event: keyEvent('b'),
            chordState: idleChordState,
            isMac: true,
            getters: noContext,
        })

        expect(row?.commandId).toBe('settings.open')
    })
})
