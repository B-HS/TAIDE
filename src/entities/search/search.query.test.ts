import { describe, expect, mock, test } from 'bun:test'
import type { SearchQuery } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createTestQueryClient, renderHookWithProviders } from '@shared/testing/render'

/**
 * `search.ipc.ts` reaches both the Tauri command bindings and `getCurrentWindow()`, neither of
 * which exists under `bun:test`, so it is stubbed before `search.query` is pulled in through a
 * *dynamic* `import()`. `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3), so this fake covers the module's whole export surface.
 */
const capturedListProjectFilesCalls: string[] = []
const capturedReplaceCalls: unknown[] = []
const replaceImpl = { current: () => Promise.resolve(0) as Promise<unknown> }

mock.module('@entities/search/search.ipc', () => ({
    runSearch: () => Promise.resolve(undefined),
    cancelSearch: () => Promise.resolve(undefined),
    replaceSearch: (input: unknown) => {
        capturedReplaceCalls.push(input)
        return replaceImpl.current()
    },
    listProjectFiles: (projectId: string) => {
        capturedListProjectFilesCalls.push(projectId)
        return Promise.resolve([])
    },
}))

const importSearchQuery = () => import('@entities/search/search.query')

const PROJECT_ID = 'project-1'

const QUERY: SearchQuery = {
    text: 'todo',
    caseSensitive: false,
    wholeWord: false,
    regex: false,
    includeGlob: null,
    excludeGlob: null,
    respectGitignore: true,
}

describe('projectFilesQueryOptions', () => {
    test('프로젝트별 SEARCH.PROJECT_FILES 키를 쓴다', async () => {
        const { projectFilesQueryOptions } = await importSearchQuery()

        expect([...projectFilesQueryOptions(PROJECT_ID).queryKey]).toEqual([...QUERY_KEY.SEARCH.PROJECT_FILES(PROJECT_ID)])
    })

    test('queryFn 이 projectId 를 그대로 IPC 에 넘긴다', async () => {
        const { projectFilesQueryOptions } = await importSearchQuery()

        await (projectFilesQueryOptions(PROJECT_ID).queryFn as () => Promise<unknown>)()

        expect(capturedListProjectFilesCalls.at(-1)).toBe(PROJECT_ID)
    })

    test('활성 프로젝트가 없으면 팩토리 자신이 enabled: false 로 막는다 (호출부 가드 불필요)', async () => {
        const { projectFilesQueryOptions } = await importSearchQuery()

        expect(projectFilesQueryOptions(null).enabled).toBe(false)
        expect([...projectFilesQueryOptions(null).queryKey]).toEqual([...QUERY_KEY.SEARCH.PROJECT_FILES('')])
    })
})

describe('useReplaceSearch', () => {
    test('입력을 그대로 IPC 로 넘기고 isPending 이 진행 상태를 대신한다', async () => {
        const { useReplaceSearch } = await importSearchQuery()
        const queryClient = createTestQueryClient()
        const input = { projectId: PROJECT_ID, query: QUERY, replacement: 'done', paths: null }

        const { result } = renderHookWithProviders(() => useReplaceSearch(), { queryClient })

        expect(result.current.isPending).toBe(false)

        await result.current.mutateAsync(input)

        expect(capturedReplaceCalls.at(-1)).toEqual(input)
    })

    test('실패를 삼키지 않고 호출부로 전파한다 (치환 실패 토스트 경로)', async () => {
        const { useReplaceSearch } = await importSearchQuery()
        const queryClient = createTestQueryClient()
        const failure = new Error('replace failed')
        replaceImpl.current = () => Promise.reject(failure)

        const { result } = renderHookWithProviders(() => useReplaceSearch(), { queryClient })

        await expect(result.current.mutateAsync({ projectId: PROJECT_ID, query: QUERY, replacement: 'done', paths: [] })).rejects.toBe(failure)

        replaceImpl.current = () => Promise.resolve(0)
    })
})
