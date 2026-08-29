import { describe, expect, test } from 'bun:test'
import { COMMIT_MESSAGE_MEMORY_LIMIT, readCommitMessageDraft, writeCommitMessageDraft } from '@entities/git/commit-message-memory'

describe('commit-message-memory', () => {
    test('뷰를 전환해 패널이 사라져도 같은 프로젝트의 메시지가 남는다', () => {
        writeCommitMessageDraft('view-switch', 'fix: 작성 중')
        expect(readCommitMessageDraft('view-switch')).toBe('fix: 작성 중')
    })

    test('프로젝트가 다르면 서로의 메시지를 보지 않는다', () => {
        writeCommitMessageDraft('project-a', 'feat: A 작업')
        expect(readCommitMessageDraft('project-b')).toBe('')

        writeCommitMessageDraft('project-b', 'feat: B 작업')
        expect(readCommitMessageDraft('project-a')).toBe('feat: A 작업')
    })

    test('빈 문자열을 쓰면 항목이 사라진다', () => {
        writeCommitMessageDraft('cleared', 'chore: 임시')
        writeCommitMessageDraft('cleared', '')
        expect(readCommitMessageDraft('cleared')).toBe('')
    })

    test('상한을 넘기면 가장 오래된 항목부터 밀려난다', () => {
        const projectIds = Array.from({ length: COMMIT_MESSAGE_MEMORY_LIMIT + 1 }, (_, index) => `limit-${index}`)
        for (const projectId of projectIds) writeCommitMessageDraft(projectId, `msg-${projectId}`)

        expect(readCommitMessageDraft(projectIds[0])).toBe('')
        expect(readCommitMessageDraft(projectIds[projectIds.length - 1])).toBe(`msg-${projectIds[projectIds.length - 1]}`)
    })

    test('다시 쓰면 최근 항목으로 올라와 상한에서 먼저 밀려나지 않는다', () => {
        const projectIds = Array.from({ length: COMMIT_MESSAGE_MEMORY_LIMIT }, (_, index) => `recency-${index}`)
        for (const projectId of projectIds) writeCommitMessageDraft(projectId, 'seed')

        writeCommitMessageDraft(projectIds[0], 'touched')
        writeCommitMessageDraft('recency-overflow', 'newest')

        expect(readCommitMessageDraft(projectIds[0])).toBe('touched')
        expect(readCommitMessageDraft(projectIds[1])).toBe('')
    })
})
