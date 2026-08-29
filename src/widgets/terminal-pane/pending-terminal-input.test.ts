import { describe, expect, test } from 'bun:test'
import { TERMINAL_PENDING_INPUT_MAX_CHARS, appendPendingTerminalInput } from '@widgets/terminal-pane/pending-terminal-input'

describe('appendPendingTerminalInput', () => {
    test('스폰 완료 전 타이핑을 순서대로 누적한다 — 유실 재현 방지', () => {
        const typed = ['l', 's', ' ', '-', 'a', '\r'].reduce((pending, key) => appendPendingTerminalInput(pending, key), '')
        expect(typed).toBe('ls -a\r')
    })

    test('빈 입력을 더해도 누적분을 보존한다', () => {
        expect(appendPendingTerminalInput('ls', '')).toBe('ls')
    })

    test('상한을 넘으면 앞에서부터 버리고 최근 입력을 남긴다', () => {
        expect(appendPendingTerminalInput('abc', 'de', 4)).toBe('bcde')
    })

    test('상한과 같은 길이는 그대로 유지한다', () => {
        expect(appendPendingTerminalInput('abc', 'd', 4)).toBe('abcd')
    })

    test('기본 상한은 TERMINAL_PENDING_INPUT_MAX_CHARS 다', () => {
        const overflowed = appendPendingTerminalInput('x'.repeat(TERMINAL_PENDING_INPUT_MAX_CHARS), 'y')
        expect(overflowed).toHaveLength(TERMINAL_PENDING_INPUT_MAX_CHARS)
        expect(overflowed.endsWith('y')).toBe(true)
    })
})
