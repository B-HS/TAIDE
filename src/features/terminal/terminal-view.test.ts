import { describe, expect, test } from 'bun:test'
import { shouldActivateTerminalLink } from '@features/terminal/terminal-view'

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
