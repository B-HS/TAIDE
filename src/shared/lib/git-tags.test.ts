import { describe, expect, test } from 'bun:test'
import type { TagInfo } from '@shared/api/bindings'
import { tagsTargetingCommit } from '@shared/lib/git-tags'

const buildTag = (overrides: Partial<TagInfo> = {}): TagInfo => ({ name: 'v1', target: 'abc', annotated: true, ...overrides })

describe('tagsTargetingCommit', () => {
    test('target 이 일치하는 태그만 반환한다', () => {
        const tags = [buildTag({ name: 'v1', target: 'abc' }), buildTag({ name: 'v2', target: 'def' })]
        expect(tagsTargetingCommit(tags, 'abc').map((tag) => tag.name)).toEqual(['v1'])
    })

    test('일치하는 태그가 없으면 빈 배열을 반환한다', () => {
        const tags = [buildTag({ name: 'v1', target: 'abc' })]
        expect(tagsTargetingCommit(tags, 'zzz')).toEqual([])
    })

    test('여러 태그가 같은 커밋을 가리키면 전부 반환한다', () => {
        const tags = [buildTag({ name: 'v1', target: 'abc' }), buildTag({ name: 'v1.0.1', target: 'abc' })]
        expect(tagsTargetingCommit(tags, 'abc').map((tag) => tag.name)).toEqual(['v1', 'v1.0.1'])
    })

    test('태그 목록이 비어 있으면 빈 배열을 반환한다', () => {
        expect(tagsTargetingCommit([], 'abc')).toEqual([])
    })
})
