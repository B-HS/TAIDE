import { describe, expect, mock, test } from 'bun:test'
import { QUERY_KEY } from '@shared/constants/query-key'
import { isGitQueryScopeMutable } from '@entities/git/git.query'

describe('isGitQueryScopeMutable', () => {
    test('rev 로 고정된 commit-files·show 스코프는 제외한다', () => {
        expect(isGitQueryScopeMutable(QUERY_KEY.GIT.COMMIT_FILES('p1', 'abc123'))).toBe(false)
        expect(isGitQueryScopeMutable(QUERY_KEY.GIT.SHOW('p1', 'abc123', 'a.txt'))).toBe(false)
    })

    test('작업트리·인덱스 상태에 의존하는 스코프는 포함한다', () => {
        expect(isGitQueryScopeMutable(QUERY_KEY.GIT.STATUS('p1'))).toBe(true)
        expect(isGitQueryScopeMutable(QUERY_KEY.GIT.LOG('p1'))).toBe(true)
        expect(isGitQueryScopeMutable(QUERY_KEY.GIT.GUTTER('p1', 'a.txt'))).toBe(true)
        expect(isGitQueryScopeMutable(QUERY_KEY.GIT.FILE_LOG('p1', 'a.txt'))).toBe(true)
    })

    test('project 키(스코프 세그먼트 없음)는 포함한다', () => {
        expect(isGitQueryScopeMutable(QUERY_KEY.GIT.PROJECT('p1'))).toBe(true)
    })

    test('blame-line·blame-overlay·conflict-sides 신설 스코프도 작업트리 의존이라 포함한다 (contract F1#17)', () => {
        expect(isGitQueryScopeMutable(['git', 'p1', 'blame-line', 'a.txt', 3])).toBe(true)
        expect(isGitQueryScopeMutable(['git', 'p1', 'blame-overlay', 'a.txt'])).toBe(true)
        expect(isGitQueryScopeMutable(['git', 'p1', 'conflict-sides', 'a.txt'])).toBe(true)
    })
})

const capturedGetGitBlameRangeCalls: { projectId: string; path: string; from: number; to: number }[] = []
const capturedGetGitConflictSidesCalls: { projectId: string; path: string }[] = []

/**
 * `@entities/git/git.ipc` re-exports Tauri command bindings (`@shared/api/bindings`) that this bun
 * test environment cannot load at import time — same constraint `tree.query.test.ts`/
 * `lsp.query.test.ts` document for their own entity's `.ipc.ts`. Stubbing the whole module, then
 * reaching `git.query.ts` through a *dynamic* `import()`, is what lets these tests exercise the new
 * `queryOptions` factories' `queryFn`s (contract F1#17) without a real backend.
 */
mock.module('@entities/git/git.ipc', () => ({
    getGitBlameRange: (input: { projectId: string; path: string; from: number; to: number }) => {
        capturedGetGitBlameRangeCalls.push(input)
        if (input.path === 'empty.ts') return Promise.resolve([])
        return Promise.resolve([{ line: input.from, commitHash: 'abc', author: 'a', authorEmail: 'a@x.com', authorTimeMs: 0, summary: 's' }])
    },
    getGitConflictSides: (input: { projectId: string; path: string }) => {
        capturedGetGitConflictSidesCalls.push(input)
        return Promise.resolve({ base: null, current: '', incoming: '' })
    },
    getGitStatus: () => Promise.resolve({ rows: [] }),
    getGitLog: () => Promise.resolve({ commits: [], hasMore: false }),
    getGitRemotes: () => Promise.resolve([]),
    getGitDiffFile: () => Promise.resolve({ hunks: [] }),
    getGitGutter: () => Promise.resolve([]),
    getGitCurrentUser: () => Promise.resolve(null),
    getGitBranches: () => Promise.resolve([]),
    getGitStashes: () => Promise.resolve([]),
    getGitTags: () => Promise.resolve([]),
    getGitCommitFiles: () => Promise.resolve([]),
    getGitFileLog: () => Promise.resolve({ commits: [], hasMore: false }),
    getGitShowFile: () => Promise.resolve({ content: '' }),
    getGitDiffStagedText: () => Promise.resolve({ diffText: '' }),
    initGitRepository: () => Promise.resolve(undefined),
    stageGitPaths: () => Promise.resolve(undefined),
    unstageGitPaths: () => Promise.resolve(undefined),
    discardGitPaths: () => Promise.resolve(undefined),
    commitGit: () => Promise.resolve(undefined),
    pushGit: () => Promise.resolve(undefined),
    pullGit: () => Promise.resolve(undefined),
    createGitBranch: () => Promise.resolve(undefined),
    checkoutGitBranch: () => Promise.resolve(undefined),
    deleteGitBranch: () => Promise.resolve(undefined),
    pushGitStash: () => Promise.resolve(undefined),
    applyGitStash: () => Promise.resolve(undefined),
    dropGitStash: () => Promise.resolve(undefined),
    discardGitHunk: () => Promise.resolve(undefined),
    resolveGitConflict: () => Promise.resolve(undefined),
    stageGitHunk: () => Promise.resolve(undefined),
    unstageGitHunk: () => Promise.resolve(undefined),
    stageGitLines: () => Promise.resolve(undefined),
    unstageGitLines: () => Promise.resolve(undefined),
    revertGitCommit: () => Promise.resolve(undefined),
    createGitTag: () => Promise.resolve(undefined),
    deleteGitTag: () => Promise.resolve(undefined),
    checkoutRemoteGitBranch: () => Promise.resolve(undefined),
}))

mock.module('@entities/ai/ai.ipc', () => ({
    cancelAiRequest: () => Promise.resolve(undefined),
    generateAiCommitMessage: () => Promise.resolve({ message: '' }),
}))

const importGitQuery = () => import('@entities/git/git.query')

describe('gitBlameLineQueryOptions (contract F1#17)', () => {
    test('projectId·path·line 이 모두 있어야 활성화된다', async () => {
        const { gitBlameLineQueryOptions } = await importGitQuery()
        expect(gitBlameLineQueryOptions({ projectId: null, path: 'a.ts', line: 3 }).enabled).toBe(false)
        expect(gitBlameLineQueryOptions({ projectId: 'p1', path: null, line: 3 }).enabled).toBe(false)
        expect(gitBlameLineQueryOptions({ projectId: 'p1', path: 'a.ts', line: null }).enabled).toBe(false)
        expect(gitBlameLineQueryOptions({ projectId: 'p1', path: 'a.ts', line: 3 }).enabled).toBe(true)
    })

    test('디바운스된 line 을 from·to 로 그대로 넘겨 첫 줄만 추출한다', async () => {
        const { gitBlameLineQueryOptions } = await importGitQuery()
        const options = gitBlameLineQueryOptions({ projectId: 'p1', path: 'a.ts', line: 7 })
        const result = await (options.queryFn as () => Promise<unknown>)()

        expect(capturedGetGitBlameRangeCalls.at(-1)).toEqual({ projectId: 'p1', path: 'a.ts', from: 7, to: 7 })
        expect(result).toMatchObject({ line: 7 })
    })

    test('결과가 없으면 null 을 반환한다', async () => {
        const { gitBlameLineQueryOptions } = await importGitQuery()
        const options = gitBlameLineQueryOptions({ projectId: 'p1', path: 'empty.ts', line: 1 })
        const result = await (options.queryFn as () => Promise<unknown>)()
        expect(result).toBeNull()
    })

    test('쿼리키는 QUERY_KEY.GIT.BLAME_LINE 중앙 팩토리와 동일하다', async () => {
        const { gitBlameLineQueryOptions } = await importGitQuery()
        expect(QUERY_KEY.GIT.BLAME_LINE('p1', 'a.ts', 7)).toEqual(gitBlameLineQueryOptions({ projectId: 'p1', path: 'a.ts', line: 7 }).queryKey)
    })
})

describe('gitBlameOverlayQueryOptions (contract F1#17)', () => {
    test('projectId·path·lineCount 이 모두 있어야 활성화된다', async () => {
        const { gitBlameOverlayQueryOptions } = await importGitQuery()
        expect(gitBlameOverlayQueryOptions({ projectId: null, path: 'a.ts', lineCount: 10 }).enabled).toBe(false)
        expect(gitBlameOverlayQueryOptions({ projectId: 'p1', path: 'a.ts', lineCount: null }).enabled).toBe(false)
        expect(gitBlameOverlayQueryOptions({ projectId: 'p1', path: 'a.ts', lineCount: 10 }).enabled).toBe(true)
    })

    test('lineCount 은 쿼리키에 포함되지 않는다 — 파일 내 줄 수 변화가 재조회를 유발하지 않는다', async () => {
        const { gitBlameOverlayQueryOptions } = await importGitQuery()
        const a = gitBlameOverlayQueryOptions({ projectId: 'p1', path: 'a.ts', lineCount: 10 }).queryKey
        const b = gitBlameOverlayQueryOptions({ projectId: 'p1', path: 'a.ts', lineCount: 999 }).queryKey
        expect(a).toEqual(b)
    })

    test('staleTime 이 0 이라 매 토글마다(enabled 재활성화 시) 재조회된다', async () => {
        const { gitBlameOverlayQueryOptions } = await importGitQuery()
        expect(gitBlameOverlayQueryOptions({ projectId: 'p1', path: 'a.ts', lineCount: 10 }).staleTime).toBe(0)
    })

    test('쿼리키는 QUERY_KEY.GIT.BLAME_OVERLAY 중앙 팩토리와 동일하다', async () => {
        const { gitBlameOverlayQueryOptions } = await importGitQuery()
        expect(QUERY_KEY.GIT.BLAME_OVERLAY('p1', 'a.ts')).toEqual(
            gitBlameOverlayQueryOptions({ projectId: 'p1', path: 'a.ts', lineCount: 10 }).queryKey,
        )
    })

    test('1 부터 lineCount 까지 전체 범위를 요청한다', async () => {
        const { gitBlameOverlayQueryOptions } = await importGitQuery()
        const options = gitBlameOverlayQueryOptions({ projectId: 'p1', path: 'a.ts', lineCount: 42 })
        await (options.queryFn as () => Promise<unknown>)()
        expect(capturedGetGitBlameRangeCalls.at(-1)).toEqual({ projectId: 'p1', path: 'a.ts', from: 1, to: 42 })
    })
})

describe('gitConflictSidesQueryOptions (contract F1#17)', () => {
    test('projectId·path 가 모두 있어야 활성화된다 — Compare 버튼이 눌리기 전엔 enabled 로만 게이팅한다', async () => {
        const { gitConflictSidesQueryOptions } = await importGitQuery()
        expect(gitConflictSidesQueryOptions({ projectId: null, path: 'a.ts' }).enabled).toBe(false)
        expect(gitConflictSidesQueryOptions({ projectId: 'p1', path: null }).enabled).toBe(false)
        expect(gitConflictSidesQueryOptions({ projectId: 'p1', path: 'a.ts' }).enabled).toBe(true)
    })

    test('queryFn 이 getGitConflictSides 를 그대로 호출한다', async () => {
        const { gitConflictSidesQueryOptions } = await importGitQuery()
        const options = gitConflictSidesQueryOptions({ projectId: 'p1', path: 'conflicted.ts' })
        await (options.queryFn as () => Promise<unknown>)()
        expect(capturedGetGitConflictSidesCalls.at(-1)).toEqual({ projectId: 'p1', path: 'conflicted.ts' })
    })

    test('쿼리키는 QUERY_KEY.GIT.CONFLICT_SIDES 중앙 팩토리와 동일하다', async () => {
        const { gitConflictSidesQueryOptions } = await importGitQuery()
        expect(QUERY_KEY.GIT.CONFLICT_SIDES('p1', 'conflicted.ts')).toEqual(
            gitConflictSidesQueryOptions({ projectId: 'p1', path: 'conflicted.ts' }).queryKey,
        )
    })
})
