import { describe, expect, mock, test } from 'bun:test'

const capturedGetTreeRowsCalls: { projectId: string; offset: number; limit: number | null }[] = []

mock.module('@entities/tree/tree.ipc', () => ({
    getTreeRows: (input: { projectId: string; offset: number; limit: number | null }) => {
        capturedGetTreeRowsCalls.push(input)
        return Promise.resolve({ rows: [], total: 0 })
    },
    refreshTreeDir: () => Promise.resolve({ rows: [], total: 0 }),
    revealTreeNode: () => Promise.resolve({ rows: [], total: 0 }),
    toggleTreeNode: () => Promise.resolve({ rows: [], total: 0 }),
}))

const importTreeQuery = () => import('@entities/tree/tree.query')

describe('treeRowsQueryOptions', () => {
    test('offset 0 에 limit: null 을 넘겨 항상 전체 트리를 요청한다 (contract §1.3(7) — Option<u32> None=전량)', async () => {
        const { treeRowsQueryOptions } = await importTreeQuery()
        const options = treeRowsQueryOptions('project-1')

        await (options.queryFn as () => Promise<unknown>)()

        expect(capturedGetTreeRowsCalls.at(-1)).toEqual({ projectId: 'project-1', offset: 0, limit: null })
    })
})
