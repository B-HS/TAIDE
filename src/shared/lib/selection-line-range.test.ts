import { describe, expect, test } from 'bun:test'
import { resolveSelectedLineRange } from '@shared/lib/selection-line-range'

describe('resolveSelectedLineRange', () => {
    test('일반적인 다중 라인 선택은 시작·끝 라인을 그대로 사용한다', () => {
        expect(resolveSelectedLineRange({ startLineNumber: 3, endLineNumber: 5, endColumn: 10 })).toEqual({ start: 3, end: 5 })
    })

    test('끝 라인이 1번째 컬럼에서 끝나면 선택되지 않은 그 라인을 제외한다', () => {
        expect(resolveSelectedLineRange({ startLineNumber: 3, endLineNumber: 6, endColumn: 1 })).toEqual({ start: 3, end: 5 })
    })

    test('단일 라인 선택은 끝 컬럼이 1이어도 시작 라인 자체를 유지한다', () => {
        expect(resolveSelectedLineRange({ startLineNumber: 3, endLineNumber: 3, endColumn: 1 })).toEqual({ start: 3, end: 3 })
    })
})
