import { describe, expect, test } from 'bun:test'
import type { StatusRow } from '@shared/api/bindings'
import { isStagedRow, isUnstagedRow, resolveCommitGate } from '@widgets/git-panel/commit-gate'

const row = (path: string, overrides: Partial<StatusRow> = {}): StatusRow => ({
    path,
    absPath: `/repo/${path}`,
    origPath: null,
    origAbsPath: null,
    staged: null,
    unstaged: null,
    isConflicted: false,
    ...overrides,
})

describe('resolveCommitGate', () => {
    test('충돌 행만 있으면 커밋을 차단한다', () => {
        expect(resolveCommitGate([row('a.ts', { unstaged: 'conflicted', isConflicted: true })])).toBe('blockedByConflicts')
    })

    test('충돌 행이 스테이지된 변경과 섞여 있어도 커밋을 차단한다', () => {
        const rows = [row('a.ts', { unstaged: 'conflicted', isConflicted: true }), row('b.ts', { staged: 'modified' })]
        expect(resolveCommitGate(rows)).toBe('blockedByConflicts')
    })

    test('스테이지된 변경이 없고 미스테이지 변경만 있으면 stage-all 확인을 요구한다', () => {
        expect(resolveCommitGate([row('a.ts', { unstaged: 'modified' })])).toBe('confirmStageAll')
    })

    test('스테이지된 변경이 있으면 바로 커밋한다', () => {
        expect(resolveCommitGate([row('a.ts', { staged: 'modified' }), row('b.ts', { unstaged: 'modified' })])).toBe('commit')
    })

    test('변경이 전혀 없으면 stage-all 로 넘어가지 않는다', () => {
        expect(resolveCommitGate([])).toBe('commit')
    })
})

describe('isStagedRow · isUnstagedRow', () => {
    test('충돌 행은 어느 그룹에도 속하지 않는다', () => {
        const conflicted = row('a.ts', { staged: 'modified', unstaged: 'conflicted', isConflicted: true })
        expect(isStagedRow(conflicted)).toBe(false)
        expect(isUnstagedRow(conflicted)).toBe(false)
    })

    test('충돌이 아닌 행은 staged·unstaged 값으로 갈린다', () => {
        expect(isStagedRow(row('a.ts', { staged: 'added' }))).toBe(true)
        expect(isUnstagedRow(row('a.ts', { staged: 'added' }))).toBe(false)
        expect(isUnstagedRow(row('b.ts', { unstaged: 'untracked' }))).toBe(true)
    })
})
