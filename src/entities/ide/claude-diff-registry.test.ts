import { describe, expect, test } from 'bun:test'
import {
    getPendingClaudeDiff,
    removePendingClaudeDiff,
    setPendingClaudeDiff,
    takePendingClaudeDiffIfUnresolved,
} from '@entities/ide/claude-diff-registry'

describe('takePendingClaudeDiffIfUnresolved', () => {
    test('아직 해결되지 않은 요청은 반환하고 레지스트리에서 제거한다', () => {
        setPendingClaudeDiff('req-1', { oldPath: '/a.ts', newContents: 'next', tabName: 'a.ts (diff)' })

        const taken = takePendingClaudeDiffIfUnresolved('req-1')

        expect(taken).toEqual({ oldPath: '/a.ts', newContents: 'next', tabName: 'a.ts (diff)' })
        expect(getPendingClaudeDiff('req-1')).toBeUndefined()
    })

    test('이미 accept/reject 로 해결되어 제거된 요청은 null 을 반환한다', () => {
        setPendingClaudeDiff('req-2', { oldPath: '/b.ts', newContents: 'next', tabName: 'b.ts (diff)' })
        removePendingClaudeDiff('req-2')

        expect(takePendingClaudeDiffIfUnresolved('req-2')).toBeNull()
    })

    test('한 번도 등록된 적 없는 요청은 null 을 반환한다', () => {
        expect(takePendingClaudeDiffIfUnresolved('req-never-registered')).toBeNull()
    })

    test('두 번 연속 호출하면 두 번째는 null 이다(원자적 소비)', () => {
        setPendingClaudeDiff('req-3', { oldPath: '/c.ts', newContents: 'next', tabName: 'c.ts (diff)' })

        expect(takePendingClaudeDiffIfUnresolved('req-3')).not.toBeNull()
        expect(takePendingClaudeDiffIfUnresolved('req-3')).toBeNull()
    })
})
