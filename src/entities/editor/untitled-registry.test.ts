import { describe, expect, test } from 'bun:test'
import { dropUntitledContent, getUntitledContent, pruneUntitledContents, setUntitledContent } from '@entities/editor/untitled-registry'

describe('untitledRegistry', () => {
    test('프로젝트별로 같은 탭 id 내용을 분리해 보관한다', () => {
        setUntitledContent('project-a', 'tab-1', 'a 내용')
        setUntitledContent('project-b', 'tab-1', 'b 내용')

        expect(getUntitledContent('project-a', 'tab-1')).toBe('a 내용')
        expect(getUntitledContent('project-b', 'tab-1')).toBe('b 내용')
    })

    test('정리는 해당 프로젝트의 탭만 대상으로 한다', () => {
        setUntitledContent('project-c', 'tab-keep', '유지')
        setUntitledContent('project-c', 'tab-drop', '삭제')
        setUntitledContent('project-d', 'tab-other', '다른 프로젝트')

        const removed = pruneUntitledContents('project-c', ['tab-keep'])

        expect(removed).toEqual(['tab-drop'])
        expect(getUntitledContent('project-c', 'tab-keep')).toBe('유지')
        expect(getUntitledContent('project-c', 'tab-drop')).toBeNull()
        expect(getUntitledContent('project-d', 'tab-other')).toBe('다른 프로젝트')
    })

    test('내용이 없는 프로젝트를 정리하면 빈 목록을 돌려준다', () => {
        expect(pruneUntitledContents('project-unknown', [])).toEqual([])
    })

    test('개별 탭 삭제는 같은 프로젝트의 다른 탭에 영향을 주지 않는다', () => {
        setUntitledContent('project-e', 'tab-1', '하나')
        setUntitledContent('project-e', 'tab-2', '둘')

        dropUntitledContent('project-e', 'tab-1')

        expect(getUntitledContent('project-e', 'tab-1')).toBeNull()
        expect(getUntitledContent('project-e', 'tab-2')).toBe('둘')
    })
})
