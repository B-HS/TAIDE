import { describe, expect, test } from 'bun:test'
import type { StatusRow } from '@shared/api/bindings'
import { buildFileTreeGitStatusByPath } from '@widgets/explorer/file-tree-git-status'

const ROOT = '/repo'

const rowOf = (absPath: string, partial: Partial<StatusRow> = {}): StatusRow => ({
    path: absPath.slice(ROOT.length + 1),
    absPath,
    isConflicted: false,
    ...partial,
})

describe('buildFileTreeGitStatusByPath', () => {
    test('unstaged 변경 종류를 파일 상태로 매핑한다', () => {
        const map = buildFileTreeGitStatusByPath([rowOf('/repo/src/a.ts', { unstaged: 'modified' })], ROOT)
        expect(map.get('/repo/src/a.ts')).toBe('modified')
    })

    test('untracked 와 typeChange 를 각각 untracked·modified 로 매핑한다', () => {
        const map = buildFileTreeGitStatusByPath(
            [rowOf('/repo/new.ts', { unstaged: 'untracked' }), rowOf('/repo/link.ts', { unstaged: 'typeChange' })],
            ROOT,
        )
        expect(map.get('/repo/new.ts')).toBe('untracked')
        expect(map.get('/repo/link.ts')).toBe('modified')
    })

    test('staged added 는 unstaged modified 보다 우선한다', () => {
        const map = buildFileTreeGitStatusByPath([rowOf('/repo/a.ts', { staged: 'added', unstaged: 'modified' })], ROOT)
        expect(map.get('/repo/a.ts')).toBe('added')
    })

    test('isConflicted 는 변경 종류와 무관하게 conflicted 다', () => {
        const map = buildFileTreeGitStatusByPath([rowOf('/repo/a.ts', { staged: 'added', unstaged: 'modified', isConflicted: true })], ROOT)
        expect(map.get('/repo/a.ts')).toBe('conflicted')
    })

    test('변경 종류가 없고 비충돌인 행은 등재하지 않는다', () => {
        const map = buildFileTreeGitStatusByPath([rowOf('/repo/a.ts')], ROOT)
        expect(map.size).toBe(0)
    })

    test('조상 디렉토리로 전파하되 프로젝트 루트 자신은 제외한다', () => {
        const map = buildFileTreeGitStatusByPath([rowOf('/repo/src/deep/a.ts', { unstaged: 'modified' })], ROOT)
        expect(map.get('/repo/src/deep')).toBe('modified')
        expect(map.get('/repo/src')).toBe('modified')
        expect(map.has('/repo')).toBe(false)
        expect(map.has('/')).toBe(false)
    })

    test('디렉토리는 자식들 중 최우선 상태를 갖는다', () => {
        const map = buildFileTreeGitStatusByPath(
            [rowOf('/repo/src/a.ts', { unstaged: 'untracked' }), rowOf('/repo/src/b.ts', { isConflicted: true })],
            ROOT,
        )
        expect(map.get('/repo/src')).toBe('conflicted')
    })

    test('형제 디렉토리의 접두사 충돌은 전파되지 않는다', () => {
        const map = buildFileTreeGitStatusByPath([rowOf('/repo-other/a.ts', { unstaged: 'modified' })], ROOT)
        expect(map.has('/repo')).toBe(false)
        expect(map.get('/repo-other/a.ts')).toBe('modified')
    })

    test('projectRoot 가 null 이면 빈 맵을 반환한다', () => {
        expect(buildFileTreeGitStatusByPath([rowOf('/repo/a.ts', { unstaged: 'modified' })], null).size).toBe(0)
    })
})
