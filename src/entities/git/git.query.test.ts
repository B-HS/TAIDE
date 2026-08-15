import { describe, expect, test } from 'bun:test'
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
})
