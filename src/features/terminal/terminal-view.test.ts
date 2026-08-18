import { describe, expect, test } from 'bun:test'
import { shouldActivateTerminalLink } from '@features/terminal/terminal-view'

describe('shouldActivateTerminalLink', () => {
    test('metaKey 만 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: false })).toBe(true)
    })

    test('altKey 만 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: true })).toBe(true)
    })

    test('둘 다 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: true })).toBe(true)
    })

    test('둘 다 눌리지 않았으면 활성화하지 않는다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: false })).toBe(false)
    })
})
