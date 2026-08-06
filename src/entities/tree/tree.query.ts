import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { getTreeRows, refreshTreeDir, revealTreeNode, toggleTreeNode } from '@entities/tree/tree.ipc'

const TREE_PAGE_LIMIT = 5_000

export const treeRowsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.TREE.ROWS(projectId ?? ''),
        queryFn: () => getTreeRows({ projectId: projectId ?? '', offset: 0, limit: TREE_PAGE_LIMIT }),
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
