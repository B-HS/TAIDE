import { describe, expect, test } from 'bun:test'
import { toHunkRange } from '@widgets/diff-pane/diff-hunk-range'

describe('toHunkRange', () => {
    test('일반 변경은 modified 라인 범위를 그대로 반환한다', () => {
        expect(toHunkRange({ modifiedStartLineNumber: 4, modifiedEndLineNumber: 6 })).toEqual({ start: 4, end: 6 })
    })

    test('순수 삭제(모든 라인이 사라짐)는 삽입 지점을 시작·끝 라인으로 사용한다', () => {
        expect(toHunkRange({ modifiedStartLineNumber: 5, modifiedEndLineNumber: 0 })).toEqual({ start: 5, end: 5 })
    })

    test('파일 맨 앞에서 순수 삭제되면 1번 라인으로 고정한다', () => {
        expect(toHunkRange({ modifiedStartLineNumber: 0, modifiedEndLineNumber: 0 })).toEqual({ start: 1, end: 1 })
    })
})
