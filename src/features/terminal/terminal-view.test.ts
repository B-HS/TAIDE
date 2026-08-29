import { describe, expect, test } from 'bun:test'
import { shouldActivateTerminalLink, shouldTranslateShiftEnterToLineFeed } from '@features/terminal/terminal-view'

describe('shouldActivateTerminalLink', () => {
    test('metaKey 만 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: false, ctrlKey: false })).toBe(true)
    })

    test('altKey 만 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: true, ctrlKey: false })).toBe(true)
    })

    test('둘 다 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: true, ctrlKey: false })).toBe(true)
    })

    test('둘 다 눌리지 않았으면 활성화하지 않는다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: false, ctrlKey: false })).toBe(false)
    })

    test('비 macOS 에서는 ctrlKey 만 눌려도 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: false, ctrlKey: true }, false)).toBe(true)
    })

    test('비 macOS 에서는 metaKey(Win 키)만 눌리면 활성화하지 않는다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: false, ctrlKey: false }, false)).toBe(false)
    })
})

describe('shouldTranslateShiftEnterToLineFeed', () => {
    const shiftEnterKeydown = {
        type: 'keydown',
        key: 'Enter',
        shiftKey: true,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        isComposing: false,
    }

    test('조합 키 없는 Shift+Enter keydown 이면 변환한다', () => {
        expect(shouldTranslateShiftEnterToLineFeed(shiftEnterKeydown)).toBe(true)
    })

    test('keydown 이 아니면(keypress·keyup) 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, type: 'keypress' })).toBe(false)
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, type: 'keyup' })).toBe(false)
    })

    test('shift 없는 Enter 는 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, shiftKey: false })).toBe(false)
    })

    test('alt·ctrl·meta 가 섞이면 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, altKey: true })).toBe(false)
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, ctrlKey: true })).toBe(false)
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, metaKey: true })).toBe(false)
    })

    test('IME 조합 중이면 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, isComposing: true })).toBe(false)
    })

    test('Enter 이외의 키는 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, key: 'a' })).toBe(false)
    })
})
