import { describe, expect, test } from 'bun:test'
import type { KeymapEntry, KeymapEvent } from '@shared/lib/keymap'
import type { KeymapChordStoreState } from '@shared/lib/keymap-chord-store'
import type { KeymapContextGetters } from '@shared/lib/keymap-context'
import { decideKeymapDispatch } from '@shared/lib/keymap-dispatch'

const noContext: KeymapContextGetters = {}
const editorFocused: KeymapContextGetters = { editorTextFocus: () => true }
const terminalFocused: KeymapContextGetters = { terminalFocus: () => true }

const idleChordState: KeymapChordStoreState = { pending: null, monacoDeferral: false }

const chordEntry: KeymapEntry = { id: 'save', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] }, descriptionKey: 'keymap.save' }
const singleEntry: KeymapEntry = { id: 'close-tab', key: 'w', mods: ['mod'], descriptionKey: 'keymap.closeTab' }
const terminalScopedEntry: KeymapEntry = {
    id: 'terminal-jump-to-previous-command',
    key: 'ArrowUp',
    mods: ['mod'],
    when: 'terminalFocus',
    descriptionKey: 'keymap.terminalJumpToPreviousCommand',
}

const keyEvent = (overrides: Partial<KeymapEvent> = {}): KeymapEvent => ({
    key: 'w',
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    ...overrides,
})

describe('decideKeymapDispatch — 일반 단일 키 매칭 (chord/when 없음, 기존 21 엔트리 행동 변화 0)', () => {
    test('일치하는 엔트리가 있으면 dispatch 를 반환한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'w', metaKey: true }), [singleEntry], idleChordState, true, noContext)
        expect(action).toEqual({ type: 'dispatch', entryId: 'close-tab' })
    })

    test('일치하는 엔트리가 없으면 none 을 반환한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'z', metaKey: true }), [singleEntry], idleChordState, true, noContext)
        expect(action).toEqual({ type: 'none' })
    })
})

describe('decideKeymapDispatch — when 게이팅', () => {
    test('when 조건이 충족되면 dispatch 된다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'ArrowUp', metaKey: true }), [terminalScopedEntry], idleChordState, true, terminalFocused)
        expect(action).toEqual({ type: 'dispatch', entryId: 'terminal-jump-to-previous-command' })
    })

    test('when 조건이 충족되지 않으면 none 을 반환한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'ArrowUp', metaKey: true }), [terminalScopedEntry], idleChordState, true, noContext)
        expect(action).toEqual({ type: 'none' })
    })
})

describe('decideKeymapDispatch — chord 1단 진입', () => {
    test('에디터 포커스가 아니면 chord 1단 진입을 반환한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [chordEntry], idleChordState, true, noContext)
        expect(action).toEqual({ type: 'enter-chord', entryId: 'save', prefix: { key: 'k', mods: ['mod'] } })
    })

    test('에디터 포커스 상태면 chord 1단 진입이 억제되고, 프리픽스가 monaco 전용(Cmd/Ctrl+K) 키와 같으면 monaco 프리픽스 관찰로 대체된다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [chordEntry], idleChordState, true, editorFocused)
        expect(action).toEqual({ type: 'observe-monaco-chord-prefix' })
    })

    test('chord 를 가진 엔트리는 pending 이 없어도 dispatch(단일 키 취급) 되지 않는다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [chordEntry], idleChordState, true, noContext)
        expect(action.type).not.toBe('dispatch')
    })
})

describe('decideKeymapDispatch — monaco 프리픽스 관찰/유예', () => {
    test('에디터 포커스 && Cmd/Ctrl+K && 앱이 소유한 chord 없음 → 관찰(monaco 에 양보, 삼키지 않음)', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [singleEntry], idleChordState, true, editorFocused)
        expect(action).toEqual({ type: 'observe-monaco-chord-prefix' })
    })

    test('에디터 포커스가 아니면 Cmd/Ctrl+K 를 관찰하지 않는다(단일 키 매칭 경로로 폴백)', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [singleEntry], idleChordState, true, noContext)
        expect(action).toEqual({ type: 'none' })
    })

    test('monacoDeferral 이 무장되어 있으면 다음(비수식키) keydown 은 defer-to-monaco 를 반환한다', () => {
        const armedState: KeymapChordStoreState = { pending: null, monacoDeferral: true }
        const action = decideKeymapDispatch(keyEvent({ key: 'b', metaKey: true }), [singleEntry], armedState, true, editorFocused)
        expect(action).toEqual({ type: 'defer-to-monaco' })
    })

    test('monacoDeferral 이 무장된 상태에서 수식키 단독 keydown 은 소비하지 않는다', () => {
        const armedState: KeymapChordStoreState = { pending: null, monacoDeferral: true }
        const action = decideKeymapDispatch(keyEvent({ key: 'Meta', metaKey: true }), [singleEntry], armedState, true, editorFocused)
        expect(action).toEqual({ type: 'ignore-modifier-only' })
    })

    test('monacoDeferral 이 무장되어 있으면 원래 매칭됐을 단일 키 엔트리도 무시하고 양보한다(⌘B 회귀 상환)', () => {
        const armedState: KeymapChordStoreState = { pending: null, monacoDeferral: true }
        const toggleSidebarEntry: KeymapEntry = { id: 'toggle-sidebar', key: 'b', mods: ['mod'], descriptionKey: 'keymap.toggleSidebar' }
        const action = decideKeymapDispatch(keyEvent({ key: 'b', metaKey: true }), [toggleSidebarEntry], armedState, true, editorFocused)
        expect(action).toEqual({ type: 'defer-to-monaco' })
    })
})

describe('decideKeymapDispatch — chord 2단 (무조건 삼킴)', () => {
    const pendingState: KeymapChordStoreState = {
        pending: { entryId: 'save', prefix: { key: 'k', mods: ['mod'] }, at: Date.now() },
        monacoDeferral: false,
    }

    test('2단 키가 일치하면 resolve-chord-matched 를 반환한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 's' }), [chordEntry], pendingState, true, noContext)
        expect(action).toEqual({ type: 'resolve-chord-matched', entryId: 'save' })
    })

    test('2단 키가 불일치해도(다른 아무 키) resolve-chord-no-match 로 무조건 삼킨다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'x' }), [chordEntry], pendingState, true, noContext)
        expect(action).toEqual({ type: 'resolve-chord-no-match' })
    })

    test('대기 중 수식키 단독 keydown 은 예외적으로 삼키지 않는다(무조건 삼킴에서 제외)', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'Shift', shiftKey: true }), [chordEntry], pendingState, true, noContext)
        expect(action).toEqual({ type: 'ignore-modifier-only' })
    })

    test('pending.entryId 에 대응하는 엔트리가 entries 에 없으면 방어적으로 no-match 처리한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 's' }), [singleEntry], pendingState, true, noContext)
        expect(action).toEqual({ type: 'resolve-chord-no-match' })
    })

    test('키 자동반복(event.repeat)은 예외적으로 삼키지 않는다(대기를 스스로 소비하지 않음)', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', repeat: true }), [chordEntry], pendingState, true, noContext)
        expect(action).toEqual({ type: 'ignore-modifier-only' })
    })

    test('IME 조합 중(event.isComposing) keydown 은 예외적으로 삼키지 않는다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'Process', isComposing: true }), [chordEntry], pendingState, true, noContext)
        expect(action).toEqual({ type: 'ignore-modifier-only' })
    })
})

describe('decideKeymapDispatch — monacoDeferral 도 자동반복/IME 조합은 소비하지 않는다', () => {
    const armedState: KeymapChordStoreState = { pending: null, monacoDeferral: true }

    test('키 자동반복은 유예를 소비하지 않는다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'b', metaKey: true, repeat: true }), [singleEntry], armedState, true, editorFocused)
        expect(action).toEqual({ type: 'ignore-modifier-only' })
    })

    test('IME 조합 중 keydown 은 유예를 소비하지 않는다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'Process', isComposing: true }), [singleEntry], armedState, true, editorFocused)
        expect(action).toEqual({ type: 'ignore-modifier-only' })
    })
})

describe('decideKeymapDispatch — 터미널 포커스 시 chord 1단 프리픽스가 억제된다(when: !terminalFocus)', () => {
    const terminalScopedChordEntry: KeymapEntry = {
        id: 'open-keybindings-editor',
        key: 'k',
        mods: ['mod'],
        chord: { key: 's', mods: ['mod'] },
        when: '!terminalFocus',
        descriptionKey: 'settings.keymapOpenEditor',
    }

    test('터미널 포커스가 아니면 평소대로 chord 1단에 진입한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [terminalScopedChordEntry], idleChordState, true, noContext)
        expect(action.type).toBe('enter-chord')
    })

    test('터미널 포커스면 chord 1단 진입이 억제되어 아무 것도 삼키지 않는다(터미널 자체의 ⌘K 관성 보존)', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [terminalScopedChordEntry], idleChordState, true, terminalFocused)
        expect(action).toEqual({ type: 'none' })
    })
})

describe('decideKeymapDispatch — monacoChordPrefixes 로 ⌘K 이외의 monaco chord 프리픽스도 유예를 무장한다', () => {
    test('기본값(monacoChordPrefixes 생략)은 Cmd/Ctrl+K 만 관찰한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'j', metaKey: true }), [singleEntry], idleChordState, true, editorFocused)
        expect(action).toEqual({ type: 'none' })
    })

    test('사용자 정의 monaco chord 프리픽스(⌘J)를 전달하면 그 키도 관찰(유예 무장)한다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'j', metaKey: true }), [singleEntry], idleChordState, true, editorFocused, [
            { key: 'j', mods: ['mod'] },
        ])
        expect(action).toEqual({ type: 'observe-monaco-chord-prefix' })
    })

    test('실제 호출부(useGlobalKeymap)처럼 기본 프리픽스와 함께 넘기면 Cmd/Ctrl+K 도 계속 관찰 대상이다', () => {
        const action = decideKeymapDispatch(keyEvent({ key: 'k', metaKey: true }), [singleEntry], idleChordState, true, editorFocused, [
            { key: 'k', mods: ['mod'] },
            { key: 'j', mods: ['mod'] },
        ])
        expect(action).toEqual({ type: 'observe-monaco-chord-prefix' })
    })
})
