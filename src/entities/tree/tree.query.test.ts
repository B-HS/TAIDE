import { describe, expect, mock, test } from 'bun:test'
import type { TreeRow, TreeRowPage } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createTestQueryClient, renderHookWithProviders, waitFor } from '@shared/testing/render'

const capturedGetTreeRowsCalls: { projectId: string; offset: number; limit: number | null }[] = []
const capturedCollapseAllCalls: { projectId: string }[] = []

const treeRow = (path: string, expanded: boolean): TreeRow => ({
    path,
    name: path.split('/').at(-1) ?? path,
    kind: 'directory',
    depth: 0,
    expanded,
    hasChildren: true,
})

const EXPANDED_PAGE: TreeRowPage = { rows: [treeRow('/project/src', true)], total: 1 }
const COLLAPSED_PAGE: TreeRowPage = { rows: [treeRow('/project/src', false)], total: 1 }

mock.module('@entities/tree/tree.ipc', () => ({
    getTreeRows: (input: { projectId: string; offset: number; limit: number | null }) => {
        capturedGetTreeRowsCalls.push(input)
        return Promise.resolve({ rows: [], total: 0 })
    },
    collapseAllTreeNodes: (input: { projectId: string }) => {
        capturedCollapseAllCalls.push(input)
        return Promise.resolve(COLLAPSED_PAGE)
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

/**
 * The "모두 접기" loop this replaced could only toggle the rows the page showed, so a directory
 * hidden under an already-collapsed parent kept its expanded flag (d-67 #24). The flag set lives in
 * Rust, so the frontend's whole job is to take exactly one `tree_collapse_all` round trip and adopt
 * the page it answers with — which is what these two lock.
 */
describe('useCollapseAllTree', () => {
    test('펼침 행이 몇 개든 tree_collapse_all 을 한 번만 호출한다', async () => {
        const { useCollapseAllTree } = await importTreeQuery()
        const queryClient = createTestQueryClient()
        const sentBefore = capturedCollapseAllCalls.length

        const { result } = renderHookWithProviders(() => useCollapseAllTree('project-1'), { queryClient })
        result.current.mutate({ projectId: 'project-1' })
        await waitFor(() => expect(capturedCollapseAllCalls.length).toBe(sentBefore + 1))

        expect(capturedCollapseAllCalls.at(-1)).toEqual({ projectId: 'project-1' })
    })

    test('응답 페이지로 TREE.ROWS 캐시를 교체한다', async () => {
        const { useCollapseAllTree } = await importTreeQuery()
        const queryClient = createTestQueryClient()
        await queryClient.fetchQuery({ queryKey: QUERY_KEY.TREE.ROWS('project-1'), queryFn: () => Promise.resolve(EXPANDED_PAGE), gcTime: Infinity })

        const { result } = renderHookWithProviders(() => useCollapseAllTree('project-1'), { queryClient })
        result.current.mutate({ projectId: 'project-1' })
        await waitFor(() => expect(queryClient.getQueryData<TreeRowPage>(QUERY_KEY.TREE.ROWS('project-1'))).toEqual(COLLAPSED_PAGE))
    })
})
