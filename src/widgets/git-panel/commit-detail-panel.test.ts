import { describe, expect, test } from 'bun:test'
import type { CommitFile, LogEntry } from '@shared/api/bindings'
import { buildCommitFileDiffOpenTabInput } from '@widgets/git-panel/commit-detail-panel'

const commit = (id: string, parents: string[]): LogEntry => ({ id, parents, summary: 's', author: 'a', timeUnix: null, refs: [] })

describe('buildCommitFileDiffOpenTabInput', () => {
    test('일반 변경 파일은 rev·parentRev·beforePath 가 동일 경로로 채워진다', () => {
        const file: CommitFile = { path: 'src/a.ts', absPath: '/repo/src/a.ts', kind: 'modified' }
        const input = buildCommitFileDiffOpenTabInput('p1', commit('0123456789abcdef', ['fedcba9876543210']), file)

        expect(input.kind).toEqual({
            kind: 'diff',
            path: 'src/a.ts',
            staged: false,
            rev: '0123456789abcdef',
            parentRev: 'fedcba9876543210',
            beforePath: 'src/a.ts',
        })
        expect(input.title).toBe('a.ts @ 0123456')
        expect(input.target).toBeNull()
        expect(input.preview).toBe(true)
    })

    test('이름 변경 파일은 beforePath 가 origPath 를 사용한다', () => {
        const file: CommitFile = {
            path: 'src/new-name.ts',
            absPath: '/repo/src/new-name.ts',
            origPath: 'src/old-name.ts',
            origAbsPath: '/repo/src/old-name.ts',
            kind: 'renamed',
        }
        const input = buildCommitFileDiffOpenTabInput('p1', commit('0123456789abcdef', ['fedcba9876543210']), file)

        expect(input.kind.beforePath).toBe('src/old-name.ts')
        expect(input.title).toBe('new-name.ts @ 0123456')
    })

    test('부모가 없는 루트 커밋은 parentRev 가 null 이다', () => {
        const file: CommitFile = { path: 'src/a.ts', absPath: '/repo/src/a.ts', kind: 'added' }
        const input = buildCommitFileDiffOpenTabInput('p1', commit('0123456789abcdef', []), file)

        expect(input.kind.parentRev).toBeNull()
    })
})
