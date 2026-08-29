import { describe, expect, test } from 'bun:test'
import type { ProblemGroup } from '@features/problems/problem-list-rows'
import { buildProblemListRows } from '@features/problems/problem-list-rows'
import type { ProblemRowData } from '@features/problems/problem-row'

const buildProblem = (overrides: Partial<ProblemRowData> = {}): ProblemRowData => ({
    severity: 'error',
    line: 1,
    column: 1,
    message: 'boom',
    source: null,
    ...overrides,
})

const buildGroup = (path: string, problems: ProblemRowData[]): ProblemGroup => ({ path, problems })

describe('buildProblemListRows', () => {
    test('그룹 헤더 다음에 그 그룹의 문제 행이 오도록 평탄화한다', () => {
        const rows = buildProblemListRows([buildGroup('/repo/a.ts', [buildProblem({ line: 3 }), buildProblem({ line: 9 })])], new Set())

        expect(rows.map((row) => row.kind)).toEqual(['group', 'problem', 'problem'])
        expect(rows[0]).toMatchObject({ kind: 'group', path: '/repo/a.ts', problemCount: 2, collapsed: false })
    })

    test('접힌 그룹은 헤더만 남기고 문제 행을 내보내지 않는다', () => {
        const rows = buildProblemListRows([buildGroup('/repo/a.ts', [buildProblem(), buildProblem()])], new Set(['/repo/a.ts']))

        expect(rows).toHaveLength(1)
        expect(rows[0]).toMatchObject({ kind: 'group', collapsed: true })
    })

    test('여러 그룹을 입력 순서대로 이어 붙이고, 접힌 그룹만 건너뛴다', () => {
        const rows = buildProblemListRows(
            [buildGroup('/repo/a.ts', [buildProblem()]), buildGroup('/repo/b.ts', [buildProblem()]), buildGroup('/repo/c.ts', [buildProblem()])],
            new Set(['/repo/b.ts']),
        )

        expect(rows.map((row) => row.id)).toEqual([
            'group:/repo/a.ts',
            'problem:/repo/a.ts:0',
            'group:/repo/b.ts',
            'group:/repo/c.ts',
            'problem:/repo/c.ts:0',
        ])
    })

    test('내용이 완전히 같은 문제가 두 건이어도 행 id 가 겹치지 않는다 (가상화 key 충돌 방지)', () => {
        const duplicate = buildProblem({ line: 7, column: 2, message: 'same' })
        const rows = buildProblemListRows([buildGroup('/repo/a.ts', [duplicate, { ...duplicate }])], new Set())

        expect(new Set(rows.map((row) => row.id)).size).toBe(rows.length)
    })

    test('그룹이 없으면 빈 배열이다', () => {
        expect(buildProblemListRows([], new Set())).toEqual([])
    })
})
