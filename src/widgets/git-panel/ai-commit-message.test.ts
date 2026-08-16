import { describe, expect, test } from 'bun:test'
import type { LogEntry } from '@shared/api/bindings'
import {
    buildRecentCommitsSummaryForAi,
    RECENT_COMMITS_FOR_AI_CONTEXT_COUNT,
    sanitizeAiCommitMessageResponse,
} from '@widgets/git-panel/ai-commit-message'

const logEntry = (id: string, summary: string): LogEntry => ({ id, parents: [], summary, author: 'a', timeUnix: null, refs: [] })

describe('buildRecentCommitsSummaryForAi', () => {
    test('짧은 해시와 summary 를 줄 단위로 조립한다', () => {
        const log = [logEntry('0123456789abcdef', 'feat: 첫 커밋'), logEntry('fedcba9876543210', 'fix: 버그 수정')]
        expect(buildRecentCommitsSummaryForAi(log)).toBe('0123456 feat: 첫 커밋\nfedcba9 fix: 버그 수정')
    })

    test(`최근 ${RECENT_COMMITS_FOR_AI_CONTEXT_COUNT}건으로 상한을 둔다`, () => {
        const log = Array.from({ length: RECENT_COMMITS_FOR_AI_CONTEXT_COUNT + 5 }, (_, index) =>
            logEntry(`hash${index}`.padEnd(10, '0'), `commit ${index}`),
        )
        expect(buildRecentCommitsSummaryForAi(log).split('\n')).toHaveLength(RECENT_COMMITS_FOR_AI_CONTEXT_COUNT)
    })

    test('빈 로그는 빈 문자열을 반환한다', () => {
        expect(buildRecentCommitsSummaryForAi([])).toBe('')
    })
})

describe('sanitizeAiCommitMessageResponse', () => {
    test('코드펜스로 감싼 응답에서 언어 태그와 펜스를 제거한다', () => {
        expect(sanitizeAiCommitMessageResponse('```\nfeat: 로그인 추가\n```')).toBe('feat: 로그인 추가')
        expect(sanitizeAiCommitMessageResponse('```text\nfeat: 로그인 추가\n```')).toBe('feat: 로그인 추가')
    })

    test('중첩 펜스는 최외곽만 벗긴다', () => {
        expect(sanitizeAiCommitMessageResponse('```\nfeat: 코드 예시 ```inline``` 포함\n```')).toBe('feat: 코드 예시 ```inline``` 포함')
    })

    test('겹따옴표·홑따옴표·유니코드 따옴표를 벗긴다', () => {
        expect(sanitizeAiCommitMessageResponse('"feat: 로그인 추가"')).toBe('feat: 로그인 추가')
        expect(sanitizeAiCommitMessageResponse("'feat: 로그인 추가'")).toBe('feat: 로그인 추가')
        expect(sanitizeAiCommitMessageResponse('“feat: 로그인 추가”')).toBe('feat: 로그인 추가')
    })

    test('펜스와 따옴표가 함께 있으면 둘 다 벗긴다', () => {
        expect(sanitizeAiCommitMessageResponse('```\n"feat: 로그인 추가"\n```')).toBe('feat: 로그인 추가')
    })

    test('앞뒤 공백을 trim 한다', () => {
        expect(sanitizeAiCommitMessageResponse('  feat: 로그인 추가  \n')).toBe('feat: 로그인 추가')
    })

    test('펜스·따옴표가 없는 일반 텍스트는 그대로 둔다', () => {
        expect(sanitizeAiCommitMessageResponse('feat: 로그인 추가')).toBe('feat: 로그인 추가')
    })

    test("작은따옴표를 포함한 문장(예: it's)에서 문장 내부 따옴표까지 제거하지 않는다", () => {
        expect(sanitizeAiCommitMessageResponse("fix: it's broken")).toBe("fix: it's broken")
    })

    test('언어 태그 없이 한 줄로 열고 닫는 펜스도 벗긴다', () => {
        expect(sanitizeAiCommitMessageResponse('```feat: add login```')).toBe('feat: add login')
    })

    test('펜스 뒤에 붙은 설명문을 버린다', () => {
        expect(sanitizeAiCommitMessageResponse('```\nfeat: 로그인 추가\n```\n\nHope this helps!')).toBe('feat: 로그인 추가')
    })
})
