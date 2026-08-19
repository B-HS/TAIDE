import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getTreeRows, refreshTreeDir, revealTreeNode, toggleTreeNode } from '@entities/tree/tree.ipc'

/**
 * `tree_rows`'s `limit` is `Option<u32>` — `None` makes Rust's `rows_page` return every row past
 * `offset`, exactly like `tree_toggle`/`tree_reveal`/`tree_refresh` already return via their own
 * `full_page` builder (contract §1.3(7), replacing the `u32::MAX` sentinel this used to pass — same
 * "always full" page shape, now expressed as the real absence of a limit instead of a magic number).
 * Before the original `u32::MAX` fix, the initial fetch truncated at a fixed 5,000-row limit while
 * every mutation replaced the same cache entry with the untruncated full tree — the cache
 * alternated between a truncated and an untruncated shape depending on which request last wrote it
 * (contract R4#12). Nothing in the frontend reads `TreeRowPage.total` for real pagination
 * (`explorer-container.tsx` just renders `page.rows`), so there is no partial-loading UX this limit
 * was serving; unifying on "always full" is the one page shape that already matches every mutation.
 */
export const treeRowsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.TREE.ROWS(projectId ?? ''),
        queryFn: () => getTreeRows({ projectId: projectId ?? '', offset: 0, limit: null }),
        enabled: !!projectId,
    })

export const useToggleTreeNode = (projectId: ProjectId | null) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: toggleTreeNode,
        onSuccess: (page) => queryClient.setQueryData(QUERY_KEY.TREE.ROWS(projectId ?? ''), page),
    })
}

export const useRevealTreeNode = (projectId: ProjectId | null) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: revealTreeNode,
        onSuccess: (page) => queryClient.setQueryData(QUERY_KEY.TREE.ROWS(projectId ?? ''), page),
    })
}

export const useRefreshTreeDir = (projectId: ProjectId | null) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: refreshTreeDir,
        onSuccess: (page) => queryClient.setQueryData(QUERY_KEY.TREE.ROWS(projectId ?? ''), page),
    })
}
