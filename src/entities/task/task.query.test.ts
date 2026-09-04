import { describe, expect, mock, test } from 'bun:test'
import { QUERY_KEY } from '@shared/constants/query-key'

/**
 * `task.ipc.ts` is a Tauri command binding that cannot load under `bun:test`, so it is stubbed
 * before `task.query` is pulled in through a *dynamic* `import()` — the pattern
 * `tree.query.test.ts` documents. `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3), so the fake covers the module's whole export surface.
 */
const capturedDetectTasksCalls: string[] = []

mock.module('@entities/task/task.ipc', () => ({
    detectTasks: (projectId: string) => {
        capturedDetectTasksCalls.push(projectId)
        return Promise.resolve([])
    },
}))

const importTaskQuery = () => import('@entities/task/task.query')

const PROJECT_ID = 'project-1'

describe('tasksQueryOptions', () => {
    test('프로젝트별 TASK.LIST 키를 쓴다', async () => {
        const { tasksQueryOptions } = await importTaskQuery()

        expect([...tasksQueryOptions(PROJECT_ID).queryKey]).toEqual([...QUERY_KEY.TASK.LIST(PROJECT_ID)])
    })

    test('queryFn 이 projectId 를 그대로 IPC 에 넘긴다', async () => {
        const { tasksQueryOptions } = await importTaskQuery()

        await (tasksQueryOptions(PROJECT_ID).queryFn as () => Promise<unknown>)()

        expect(capturedDetectTasksCalls.at(-1)).toBe(PROJECT_ID)
    })

    test('활성 프로젝트가 없으면 enabled 가 false 라 호출부가 별도 가드를 두지 않아도 된다', async () => {
        const { tasksQueryOptions } = await importTaskQuery()

        expect(tasksQueryOptions(null).enabled).toBe(false)
        expect(tasksQueryOptions(PROJECT_ID).enabled).toBe(true)
    })

    test('projectId 가 null 이면 키의 마지막 세그먼트가 빈 문자열이라 실제 프로젝트 캐시와 섞이지 않는다', async () => {
        const { tasksQueryOptions } = await importTaskQuery()

        expect([...tasksQueryOptions(null).queryKey]).toEqual([...QUERY_KEY.TASK.LIST('')])
    })
})
