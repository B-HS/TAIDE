import { describe, expect, test } from 'bun:test'
import type { StatusRow } from '@shared/api/bindings'
import { isDiffHunkStageable } from '@widgets/diff-pane/diff-stageability'

const conflictedRow: StatusRow = {
    path: 'src/a.ts',
    absPath: '/repo/src/a.ts',
    origPath: null,
    origAbsPath: null,
    staged: null,
    unstaged: 'conflicted',
    isConflicted: true,
}

describe('isDiffHunkStageable', () => {
    test('충돌 파일의 절대경로 diff 탭은 hunk 스테이지를 제공하지 않는다', () => {
        expect(isDiffHunkStageable({ path: '/repo/src/a.ts', compareWith: null, rows: [conflictedRow] })).toBe(false)
    })

    test('옛 레이아웃이 남긴 상대경로 diff 탭도 충돌을 인식한다', () => {
        expect(isDiffHunkStageable({ path: 'src/a.ts', compareWith: null, rows: [conflictedRow] })).toBe(false)
    })

    test('다른 파일의 충돌은 이 탭의 판정에 영향을 주지 않는다', () => {
        expect(isDiffHunkStageable({ path: '/repo/src/b.ts', compareWith: null, rows: [conflictedRow] })).toBe(true)
    })

    test('충돌이 아닌 파일은 hunk 스테이지를 제공한다', () => {
        const rows: StatusRow[] = [{ ...conflictedRow, unstaged: 'modified', isConflicted: false }]
        expect(isDiffHunkStageable({ path: '/repo/src/a.ts', compareWith: null, rows })).toBe(true)
    })

    test('파일 대 파일 비교는 언제나 hunk 스테이지를 제공하지 않는다', () => {
        expect(isDiffHunkStageable({ path: '/repo/src/a.ts', compareWith: '/repo/src/b.ts', rows: [] })).toBe(false)
    })
})
