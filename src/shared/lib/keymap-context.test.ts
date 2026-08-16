import { describe, expect, test } from 'bun:test'
import { getKeymapContextValue } from '@shared/lib/keymap-context'

/**
 * `DEFAULT_KEYMAP_CONTEXT_GETTERS` itself reads `document.activeElement`, which requires a real
 * DOM — like `command-relay.ts`'s `executeMonacoCommand` (see that file's doc comment), it is
 * unverified by unit test for the same reason (`bun:test` has no `document`). Only the pure
 * lookup logic (`getKeymapContextValue`) is tested here, against injected fake getters — matching
 * "게터 맵은 테스트 주입 가능 구조" (Wave H contract §3.2).
 */
describe('getKeymapContextValue', () => {
    test('주입된 게터가 true 를 반환하면 true 를 반환한다', () => {
        expect(getKeymapContextValue('terminalFocus', { terminalFocus: () => true })).toBe(true)
    })

    test('주입된 게터가 false 를 반환하면 false 를 반환한다', () => {
        expect(getKeymapContextValue('terminalFocus', { terminalFocus: () => false })).toBe(false)
    })

    test('게터 맵에 없는 키는 false 로 취급한다', () => {
        expect(getKeymapContextValue('unknownKey', {})).toBe(false)
    })

    test('여러 게터가 있는 맵에서 요청한 키의 게터만 호출한다', () => {
        let terminalFocusCalls = 0
        const getters = {
            editorTextFocus: () => true,
            terminalFocus: () => {
                terminalFocusCalls += 1
                return false
            },
        }
        expect(getKeymapContextValue('editorTextFocus', getters)).toBe(true)
        expect(terminalFocusCalls).toBe(0)
    })
})
