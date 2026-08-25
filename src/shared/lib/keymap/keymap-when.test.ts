import { describe, expect, test } from 'bun:test'
import { APP_KEYMAP } from '@shared/lib/keymap/keymap'
import { DEFAULT_KEYMAP_CONTEXT_GETTERS } from '@shared/lib/keymap/keymap-context'
import { evaluateKeymapWhen } from '@shared/lib/keymap/keymap-when'

/**
 * Wave H contract §3.2 load gate: verifies monaco's `ContextKeyExpr.deserialize`/`evaluate` load
 * and run under `bun:test` (no DOM) via the exact deep-import path `keymap-when.ts` uses. If a
 * monaco upgrade ever breaks this, this test — not a downstream `when`-evaluation test — is the
 * one that should fail first, pointing straight at the load gate rather than a matching bug.
 */
describe('monaco ContextKeyExpr load gate', () => {
    test('deserialize + evaluate 가 bun:test(DOM 없는) 환경에서 정상 동작한다', async () => {
        const { ContextKeyExpr } = await import('monaco-editor/platform/contextkey/common/contextkey')
        const expr = ContextKeyExpr.deserialize('a && !b')
        expect(expr).toBeDefined()
        expect(expr?.evaluate({ getValue: (key) => ({ a: true, b: false })[key] })).toBe(true)
        expect(expr?.evaluate({ getValue: (key) => ({ a: true, b: true })[key] })).toBe(false)
    })
})

describe('evaluateKeymapWhen', () => {
    test('when 이 undefined 면 게터 없이도 항상 true 다', () => {
        expect(evaluateKeymapWhen(undefined)).toBe(true)
        expect(evaluateKeymapWhen(undefined, {})).toBe(true)
    })

    test('단일 컨텍스트 키를 주입된 게터로 평가한다', () => {
        expect(evaluateKeymapWhen('terminalFocus', { terminalFocus: () => true })).toBe(true)
        expect(evaluateKeymapWhen('terminalFocus', { terminalFocus: () => false })).toBe(false)
    })

    test('!(부정) 표현식을 평가한다', () => {
        expect(evaluateKeymapWhen('!editorTextFocus', { editorTextFocus: () => false })).toBe(true)
        expect(evaluateKeymapWhen('!editorTextFocus', { editorTextFocus: () => true })).toBe(false)
    })

    test('&&(그리고) 표현식을 평가한다', () => {
        const getters = { terminalFocus: () => true, editorTextFocus: () => false }
        expect(evaluateKeymapWhen('terminalFocus && !editorTextFocus', getters)).toBe(true)
        expect(evaluateKeymapWhen('terminalFocus && editorTextFocus', getters)).toBe(false)
    })

    test('||(또는) 표현식을 평가한다', () => {
        const getters = { terminalFocus: () => false, editorTextFocus: () => true }
        expect(evaluateKeymapWhen('terminalFocus || editorTextFocus', getters)).toBe(true)
    })

    test('게터 맵에 없는 컨텍스트 키는 false 로 취급한다', () => {
        expect(evaluateKeymapWhen('unknownContextKey', {})).toBe(false)
    })

    test('빈 문자열은 무효 표현식으로 취급해 false 를 반환한다(엔트리 비활성)', () => {
        expect(evaluateKeymapWhen('', { terminalFocus: () => true })).toBe(false)
    })

    test('문법이 깨진 표현식은 false 를 반환한다(엔트리 비활성 — 항상 매칭으로 폴백하지 않는다)', () => {
        expect(evaluateKeymapWhen('terminalFocus &&', { terminalFocus: () => true })).toBe(false)
    })

    test('동일한 when 문자열을 반복 평가해도 매번 일관된 결과를 낸다(캐시 경로 확인)', () => {
        const getters = { terminalFocus: () => true }
        expect(evaluateKeymapWhen('terminalFocus', getters)).toBe(true)
        expect(evaluateKeymapWhen('terminalFocus', getters)).toBe(true)
        expect(evaluateKeymapWhen('terminalFocus', { terminalFocus: () => false })).toBe(false)
    })
})

/**
 * A malformed `when` (typo, syntax error, or a reference to a context key that's never in the
 * `DEFAULT_KEYMAP_CONTEXT_GETTERS` whitelist) doesn't throw or warn anywhere — it silently disables
 * the entry (`evaluateKeymapWhen`'s documented "unparseable → false" stance, and an unknown context
 * key reads as `false` too — see `getKeymapContextValue`). Without this dev-time guard, that entry
 * would just never fire and nothing in the codebase would say why.
 */
describe('APP_KEYMAP 의 when 필드는 전부 유효하고 화이트리스트 컨텍스트 키만 참조한다', () => {
    test('when 을 가진 모든 엔트리는 deserialize 에 성공하고, 참조하는 컨텍스트 키가 전부 화이트리스트 안에 있다', async () => {
        const { ContextKeyExpr } = await import('monaco-editor/platform/contextkey/common/contextkey')
        const whitelist = new Set(Object.keys(DEFAULT_KEYMAP_CONTEXT_GETTERS))
        const whenStrings = APP_KEYMAP.map((entry) => entry.when).filter((when): when is string => when !== undefined)
        expect(whenStrings.length).toBeGreaterThan(0)
        for (const when of whenStrings) {
            const expression = ContextKeyExpr.deserialize(when)
            expect(expression).toBeDefined()
            for (const key of expression?.keys() ?? []) expect(whitelist.has(key)).toBe(true)
        }
    })
})
