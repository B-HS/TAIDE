import { describe, expect, test } from 'bun:test'
import { TERMINAL_ANSI_TOKENS } from '@shared/lib/theme-convert/types'
import { TERMINAL_TOKENS } from '@entities/theme/theme-tokens'

describe('TERMINAL_TOKENS', () => {
    test('앞 16개 ANSI 이름이 TERMINAL_ANSI_TOKENS 와 순서까지 일치한다', () => {
        expect(TERMINAL_TOKENS.slice(0, TERMINAL_ANSI_TOKENS.length)).toEqual([...TERMINAL_ANSI_TOKENS])
    })
})
