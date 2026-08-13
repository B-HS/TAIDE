import { describe, expect, test } from 'bun:test'
import { buildMonacoKeybinding, isMonacoCommandId, resolveMonacoKeyCode, toMonacoActionId } from '@shared/lib/monaco-keybinding'

describe('resolveMonacoKeyCode', () => {
    test('알파벳·숫자 키를 KeyCode 값으로 변환한다', () => {
        expect(resolveMonacoKeyCode('f')).toBe(36)
        expect(resolveMonacoKeyCode('0')).toBe(21)
    })

    test('대소문자를 구분하지 않는다', () => {
        expect(resolveMonacoKeyCode('F')).toBe(resolveMonacoKeyCode('f'))
    })

    test('기능키·화살표·구두점을 변환한다', () => {
        expect(resolveMonacoKeyCode('F12')).toBe(70)
        expect(resolveMonacoKeyCode('ArrowUp')).toBe(16)
        expect(resolveMonacoKeyCode('\\')).toBe(93)
    })

    test('알 수 없는 키는 null 을 반환한다', () => {
        expect(resolveMonacoKeyCode('Unknown')).toBeNull()
    })
})

describe('buildMonacoKeybinding', () => {
    test('mod 는 CtrlCmd 비트(2048)를 더한다', () => {
        expect(buildMonacoKeybinding('f', ['mod'])).toBe(2048 | 36)
    })

    test('ctrl 은 WinCtrl 비트(256)를 더한다', () => {
        expect(buildMonacoKeybinding('g', ['ctrl'])).toBe(256 | 37)
    })

    test('여러 modifier 를 비트 OR 로 조합한다', () => {
        expect(buildMonacoKeybinding('l', ['mod', 'shift'])).toBe(2048 | 1024 | 42)
    })

    test('modifier 없이 키만 지정할 수 있다', () => {
        expect(buildMonacoKeybinding('F12', [])).toBe(70)
    })

    test('알 수 없는 키면 null 을 반환한다', () => {
        expect(buildMonacoKeybinding('Unknown', ['mod'])).toBeNull()
    })
})

describe('monaco 커맨드 id 유틸', () => {
    test('isMonacoCommandId 는 monaco. 접두사를 판별한다', () => {
        expect(isMonacoCommandId('monaco.editor.action.rename')).toBe(true)
        expect(isMonacoCommandId('editor.save')).toBe(false)
    })

    test('toMonacoActionId 는 접두사를 제거한다', () => {
        expect(toMonacoActionId('monaco.editor.action.rename')).toBe('editor.action.rename')
    })

    test('toMonacoActionId 는 접두사가 없으면 그대로 반환한다', () => {
        expect(toMonacoActionId('editor.action.rename')).toBe('editor.action.rename')
    })
})
