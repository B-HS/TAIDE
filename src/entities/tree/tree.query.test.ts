import { describe, expect, mock, test } from 'bun:test'

const capturedGetTreeRowsCalls: { projectId: string; offset: number; limit: number }[] = []

mock.module('@entities/tree/tree.ipc', () => ({
    getTreeRows: (input: { projectId: string; offset: number; limit: number }) => {
        capturedGetTreeRowsCalls.push(input)
        return Promise.resolve({ rows: [], total: 0 })
    },
    refreshTreeDir: () => Promise.resolve({ rows: [], total: 0 }),
    revealTreeNode: () => Promise.resolve({ rows: [], total: 0 }),
    toggleTreeNode: () => Promise.resolve({ rows: [], total: 0 }),
}))

const importTreeQuery = () => import('@entities/tree/tree.query')

describe('treeRowsQueryOptions', () => {
    test('offset 0 에 u32::MAX 를 limit 으로 넘겨 항상 전체 트리를 요청한다 (contract R4#12)', async () => {
        const { treeRowsQueryOptions, TREE_ROWS_UNBOUNDED_LIMIT } = await importTreeQuery()
        const options = treeRowsQueryOptions('project-1')

        await (options.queryFn as () => Promise<unknown>)()

        expect(capturedGetTreeRowsCalls.at(-1)).toEqual({ projectId: 'project-1', offset: 0, limit: TREE_ROWS_UNBOUNDED_LIMIT })
    })

    test('mutation 응답이 반환하는 full_page 와 같은 무제한 계약을 쓴다 — 절단↔전량을 오가지 않는다', async () => {
        const { TREE_ROWS_UNBOUNDED_LIMIT } = await importTreeQuery()
        expect(TREE_ROWS_UNBOUNDED_LIMIT).toBe(4_294_967_295)
    })
})
