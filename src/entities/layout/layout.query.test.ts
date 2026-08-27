import { describe, expect, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import type { PaneNode, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { applyFreshLayout } from '@entities/layout/layout.query'

const EMPTY_ROOT: PaneNode = { node: 'leaf', id: 'leaf', tabs: [], active: null }

const buildLayout = (revision: number): ProjectLayout => ({ version: 1, root: EMPTY_ROOT, focusedPane: 'leaf', revision })

/**
 * Reproduces the d-42 dirty-dot persistence bug (contract §3, item b): a `setTabDirty` mutation's
 * IPC response can resolve *after* a later mutation's response despite starting first (frontend
 * `invoke()` settlement order isn't bound to call order — see `applyFreshLayout`'s doc comment).
 * Before the fix, `onSuccess` wrote every mutation response straight into the cache with no
 * ordering check, so this exact sequence would leave the *stale* `dirty:true` layout sitting in the
 * cache even though the fresher `dirty:false` save already landed.
 */
describe('applyFreshLayout', () => {
    test('최신 revision 을 먼저 받고 이후 stale 한 이전 revision 응답이 도착해도 캐시를 덮어쓰지 않는다 (out-of-order 저장 응답)', () => {
        const queryClient = new QueryClient()
        const projectId = 'project-1'
        const key = QUERY_KEY.LAYOUT.DETAIL(projectId)

        applyFreshLayout(queryClient, projectId, buildLayout(2))
        expect(queryClient.getQueryData<ProjectLayout>(key)?.revision).toBe(2)

        applyFreshLayout(queryClient, projectId, buildLayout(1))
        expect(queryClient.getQueryData<ProjectLayout>(key)?.revision).toBe(2)
    })

    test('캐시가 비어 있으면(첫 쓰기) 어떤 revision 이든 그대로 반영한다', () => {
        const queryClient = new QueryClient()
        const projectId = 'project-1'
        const key = QUERY_KEY.LAYOUT.DETAIL(projectId)

        applyFreshLayout(queryClient, projectId, buildLayout(0))

        expect(queryClient.getQueryData<ProjectLayout>(key)?.revision).toBe(0)
    })

    test('더 높은 revision 이 도착하면 정상적으로 갱신한다', () => {
        const queryClient = new QueryClient()
        const projectId = 'project-1'
        const key = QUERY_KEY.LAYOUT.DETAIL(projectId)

        applyFreshLayout(queryClient, projectId, buildLayout(1))
        applyFreshLayout(queryClient, projectId, buildLayout(2))

        expect(queryClient.getQueryData<ProjectLayout>(key)?.revision).toBe(2)
    })
})
