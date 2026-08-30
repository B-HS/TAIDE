import { describe, expect, test } from 'bun:test'
import { focusTextInputCaretAtEnd } from '@shared/lib/text-input-caret'

/**
 * Stands in for the real input so the caret rule is checked without a DOM: it records the calls the
 * helper makes and mimics the select-all a programmatic focus leaves behind (radix's
 * `focus(element, { select: true })`, and WKWebView's cached-selection restore) so the assertions
 * see the state the helper has to overwrite rather than a blank one.
 */
const createFocusSelectAllInput = (value: string) => {
    const state: { calls: string[]; selection: [number, number] | null } = { calls: [], selection: null }

    return {
        state,
        value,
        focus: (options?: FocusOptions) => {
            state.calls = [...state.calls, `focus:${options?.preventScroll === true}`]
            state.selection = [0, value.length]
        },
        setSelectionRange: (start: number | null, end: number | null) => {
            state.calls = [...state.calls, 'setSelectionRange']
            state.selection = [start ?? 0, end ?? 0]
        },
    }
}

describe('focusTextInputCaretAtEnd', () => {
    test('프리픽스가 채워진 입력의 전체선택을 끝 캐럿으로 되돌린다', () => {
        const input = createFocusSelectAllInput('>')
        focusTextInputCaretAtEnd(input)
        expect(input.state.selection).toEqual([1, 1])
    })

    test('빈 쿼리로 열려도 캐럿을 0 에 둔다', () => {
        const input = createFocusSelectAllInput('')
        focusTextInputCaretAtEnd(input)
        expect(input.state.selection).toEqual([0, 0])
    })

    test('이미 입력된 쿼리는 지우지 않고 끝으로만 접는다', () => {
        const input = createFocusSelectAllInput('@symbol')
        focusTextInputCaretAtEnd(input)
        expect(input.state.selection).toEqual([7, 7])
    })

    test('포커스를 먼저 준 뒤 캐럿을 접는다 — 순서가 뒤바뀌면 전체선택이 남는다', () => {
        const input = createFocusSelectAllInput('>')
        focusTextInputCaretAtEnd(input)
        expect(input.state.calls).toEqual(['focus:true', 'setSelectionRange'])
    })

    test('입력이 아직 마운트되지 않았으면 아무것도 하지 않는다', () => {
        expect(() => focusTextInputCaretAtEnd(null)).not.toThrow()
    })
})
