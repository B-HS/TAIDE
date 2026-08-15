import { describe, expect, test } from 'bun:test'
import {
    clampContextLines,
    SEARCH_EDITOR_MAX_CONTEXT_LINES,
    SEARCH_EDITOR_MIN_CONTEXT_LINES,
} from '@widgets/search-editor/search-editor-context-lines'

describe('clampContextLines', () => {
    test('범위 안의 값은 그대로 반환한다', () => {
        expect(clampContextLines(3)).toBe(3)
    })

    test('최댓값을 넘으면 최댓값으로 자른다', () => {
        expect(clampContextLines(SEARCH_EDITOR_MAX_CONTEXT_LINES + 5)).toBe(SEARCH_EDITOR_MAX_CONTEXT_LINES)
    })

    test('음수는 최솟값으로 자른다', () => {
        expect(clampContextLines(-1)).toBe(SEARCH_EDITOR_MIN_CONTEXT_LINES)
    })

    test('소수는 정수로 잘라낸다', () => {
        expect(clampContextLines(2.9)).toBe(2)
    })

    test('NaN 은 최솟값으로 취급한다', () => {
        expect(clampContextLines(Number.NaN)).toBe(SEARCH_EDITOR_MIN_CONTEXT_LINES)
    })
})
