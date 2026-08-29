import { describe, expect, test } from 'bun:test'
import type { SearchEditorFormState } from '@entities/search/search-editor-memory'
import { readSearchEditorMemory, SEARCH_EDITOR_MEMORY_LIMIT, writeSearchEditorMemory } from '@entities/search/search-editor-memory'

const form = (overrides: Partial<SearchEditorFormState> = {}): SearchEditorFormState => ({
    queryText: 'needle',
    caseSensitive: false,
    wholeWord: false,
    regex: false,
    respectGitignore: true,
    excludeGlob: '',
    contextLines: 0,
    ...overrides,
})

const entry = (projectId: string, overrides: Partial<SearchEditorFormState> = {}) => ({
    projectId,
    form: form(overrides),
    run: {
        groups: [{ path: 'src/a.ts', matches: [] }],
        totalMatches: 1,
        status: 'completed' as const,
        query: { text: 'needle' },
    },
})

describe('search-editor-memory', () => {
    test('저장한 적 없는 탭은 비어 있다', () => {
        expect(readSearchEditorMemory('tab-unknown', 'project-1')).toBeNull()
    })

    test('탭을 떠날 때 저장한 입력과 결과를 그대로 돌려준다', () => {
        writeSearchEditorMemory('tab-restore', entry('project-1', { queryText: 'edited' }))

        const restored = readSearchEditorMemory('tab-restore', 'project-1')

        expect(restored?.form.queryText).toBe('edited')
        expect(restored?.run.totalMatches).toBe(1)
        expect(restored?.run.groups.map((group) => group.path)).toEqual(['src/a.ts'])
    })

    test('다른 프로젝트의 항목은 돌려주지 않고 버린다', () => {
        writeSearchEditorMemory('tab-cross', entry('project-1'))

        expect(readSearchEditorMemory('tab-cross', 'project-2')).toBeNull()
        expect(readSearchEditorMemory('tab-cross', 'project-1')).toBeNull()
    })

    test('상한을 넘으면 가장 오래 쓰지 않은 탭부터 버린다', () => {
        const tabIds = Array.from({ length: SEARCH_EDITOR_MEMORY_LIMIT + 1 }, (_, index) => `tab-lru-${index}`)
        for (const tabId of tabIds) writeSearchEditorMemory(tabId, entry('project-lru'))

        expect(readSearchEditorMemory(tabIds[0], 'project-lru')).toBeNull()
        expect(readSearchEditorMemory(tabIds[tabIds.length - 1], 'project-lru')).not.toBeNull()
    })

    test('다시 저장한 탭은 최신으로 취급되어 상한에서 먼저 밀려나지 않는다', () => {
        const tabIds = Array.from({ length: SEARCH_EDITOR_MEMORY_LIMIT }, (_, index) => `tab-touch-${index}`)
        for (const tabId of tabIds) writeSearchEditorMemory(tabId, entry('project-touch'))

        writeSearchEditorMemory(tabIds[0], entry('project-touch', { queryText: 'touched' }))
        writeSearchEditorMemory('tab-touch-extra', entry('project-touch'))

        expect(readSearchEditorMemory(tabIds[0], 'project-touch')?.form.queryText).toBe('touched')
        expect(readSearchEditorMemory(tabIds[1], 'project-touch')).toBeNull()
    })
})
