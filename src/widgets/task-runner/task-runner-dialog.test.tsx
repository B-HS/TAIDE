import { describe, expect, test } from 'bun:test'
import type { Task } from '@shared/api/bindings'
import { requestOpenTaskRunner } from '@shared/lib/bridge/task-runner-bridge'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderWithProviders, screen } from '@shared/testing/render'
import { TaskRunnerDialog } from '@widgets/task-runner/task-runner-dialog'

/**
 * The dialog used to discover tasks for whatever the *global* active-project session pointed at,
 * which is why it could only be mounted in the main window. d-62 §1.D gave it a `projectId` prop
 * instead, so an auxiliary window can mount its own copy pinned to its own project — that is what
 * these cases lock: the task list is read under the prop's project, and never under another one.
 */
const PROJECT_ID = 'project-1'
const OTHER_PROJECT_ID = 'project-2'

const TASKS: Task[] = [{ label: 'build', command: 'bun run build', source: 'npm', cwd: '/tmp/aux-project' }]

/** Seeded with `gcTime: Infinity` because the test client collects observer-less queries immediately (`docs/memory/test-conventions.md` §3). */
const renderDialog = async (projectId: string) => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.TASK.LIST(PROJECT_ID), queryFn: () => TASKS, gcTime: Infinity })
    const rendered = renderWithProviders(<TaskRunnerDialog projectId={projectId} />, { queryClient })
    act(() => requestOpenTaskRunner())
    return rendered
}

describe('TaskRunnerDialog 프로젝트 스코프', () => {
    test('prop 으로 받은 프로젝트의 태스크를 보여준다', async () => {
        await renderDialog(PROJECT_ID)

        expect(await screen.findByText(TASKS[0].label)).toBeTruthy()
    })

    test('다른 프로젝트를 받으면 그 프로젝트의 태스크 목록만 조회한다', async () => {
        const { queryClient } = await renderDialog(OTHER_PROJECT_ID)

        expect(screen.queryByText(TASKS[0].label)).toBeNull()
        expect(queryClient.getQueryState(QUERY_KEY.TASK.LIST(OTHER_PROJECT_ID))).toBeTruthy()
    })

    test('프로젝트가 없으면 태스크를 조회하지 않는다', async () => {
        const queryClient = createTestQueryClient()
        renderWithProviders(<TaskRunnerDialog projectId={null} />, { queryClient })

        act(() => requestOpenTaskRunner())

        expect(queryClient.getQueryState(QUERY_KEY.TASK.LIST(''))?.fetchStatus).not.toBe('fetching')
    })
})
