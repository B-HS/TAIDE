import { describe, expect, test } from 'bun:test'
import { MONACO_ACTIONS, TAIDE_CUSTOM_ACTIONS } from '@shared/lib/monaco/monaco-actions'
import { parseMonacoDefaultBindingLabel } from '@shared/lib/monaco/monaco-binding-label'

describe('parseMonacoDefaultBindingLabel', () => {
    test('수식 글리프와 키를 분해한다', () => {
        expect(parseMonacoDefaultBindingLabel('⌘D')).toEqual({ key: 'd', mods: ['mod'] })
        expect(parseMonacoDefaultBindingLabel('⌥⇧A')).toEqual({ key: 'a', mods: ['alt', 'shift'] })
        expect(parseMonacoDefaultBindingLabel('⌃K')).toEqual({ key: 'k', mods: ['ctrl'] })
    })

    test('2단 chord 는 공백으로 나눠 chord 로 담는다', () => {
        expect(parseMonacoDefaultBindingLabel('⌘K ⌘C')).toEqual({ key: 'k', mods: ['mod'], chord: { key: 'c', mods: ['mod'] } })
        expect(parseMonacoDefaultBindingLabel('⌘K F12')).toEqual({ key: 'k', mods: ['mod'], chord: { key: 'F12', mods: [] } })
    })

    test('수식 없는 F 키와 이름 있는 키도 처리한다', () => {
        expect(parseMonacoDefaultBindingLabel('F12')).toEqual({ key: 'F12', mods: [] })
        expect(parseMonacoDefaultBindingLabel('⇧F10')).toEqual({ key: 'F10', mods: ['shift'] })
        expect(parseMonacoDefaultBindingLabel('⌘⌫')).toEqual({ key: 'Backspace', mods: ['mod'] })
        expect(parseMonacoDefaultBindingLabel('⌃Space')).toEqual({ key: 'space', mods: ['ctrl'] })
    })

    test('라벨이 없으면 null 을 반환한다', () => {
        expect(parseMonacoDefaultBindingLabel(null)).toBeNull()
        expect(parseMonacoDefaultBindingLabel('⌘')).toBeNull()
    })

    test('카탈로그의 모든 기본 바인딩 라벨이 파싱된다', () => {
        const unparsed = [...MONACO_ACTIONS, ...TAIDE_CUSTOM_ACTIONS]
            .filter((entry) => entry.defaultBindingLabel !== null && parseMonacoDefaultBindingLabel(entry.defaultBindingLabel) === null)
            .map((entry) => entry.actionId)
        expect(unparsed).toEqual([])
    })
})
