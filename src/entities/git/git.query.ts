import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { DiffMode, ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import {
    applyGitStash,
    checkoutGitBranch,
    commitGit,
    createGitBranch,
    deleteGitBranch,
    discardGitHunk,
    discardGitPaths,
    dropGitStash,
    getGitBranches,
    getGitStashes,
    pushGitStash,
    getGitCurrentUser,
    getGitDiffFile,
    getGitGutter,
    getGitLog,
    getGitRemotes,
    getGitStatus,
    pullGit,
    pushGit,
    stageGitPaths,
    unstageGitPaths,
} from '@entities/git/git.ipc'

const LOG_PAGE_SIZE = 100

export const gitStatusQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.STATUS(projectId ?? ''),
        queryFn: () => getGitStatus(projectId ?? ''),
        enabled: !!projectId,
        retry: false,
    })

export const gitLogQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.LOG(projectId ?? ''),
        queryFn: () => getGitLog({ projectId: projectId ?? '', skip: 0, take: LOG_PAGE_SIZE }),
        enabled: !!projectId,
        retry: false,
    })

export const gitRemotesQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.REMOTES(projectId ?? ''),
        queryFn: () => getGitRemotes(projectId ?? ''),
        enabled: !!projectId,
        retry: false,
    })

export const gitDiffFileQueryOptions = (input: { projectId: ProjectId | null; path: string | null; mode: DiffMode }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.DIFF(input.projectId ?? '', input.path ?? '', input.mode),
        queryFn: () => getGitDiffFile({ projectId: input.projectId ?? '', path: input.path ?? '', mode: input.mode }),
        enabled: !!input.projectId && !!input.path,
        retry: false,
    })

export const gitGutterQueryOptions = (input: { projectId: ProjectId | null; path: string | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.GUTTER(input.projectId ?? '', input.path ?? ''),
        queryFn: () => getGitGutter({ projectId: input.projectId ?? '', path: input.path ?? '' }),
        enabled: !!input.projectId && !!input.path,
        retry: false,
    })

export const gitCurrentUserQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.CURRENT_USER(projectId ?? ''),
        queryFn: () => getGitCurrentUser(projectId ?? ''),
        enabled: !!projectId,
        staleTime: Infinity,
        retry: false,
    })

const useGitMutation = <TVariables, TResult>(projectId: ProjectId | null, mutationFn: (variables: TVariables) => Promise<TResult>) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId ?? '') }),
    })
}

export const useStageGitPaths = (projectId: ProjectId | null) => useGitMutation(projectId, stageGitPaths)

export const useUnstageGitPaths = (projectId: ProjectId | null) => useGitMutation(projectId, unstageGitPaths)

export const useDiscardGitPaths = (projectId: ProjectId | null) => useGitMutation(projectId, discardGitPaths)

export const useCommitGit = (projectId: ProjectId | null) => useGitMutation(projectId, commitGit)

export const usePushGit = (projectId: ProjectId | null) => useGitMutation(projectId, pushGit)

export const usePullGit = (projectId: ProjectId | null) => useGitMutation(projectId, pullGit)

export const gitBranchesQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.BRANCHES(projectId ?? ''),
        queryFn: () => getGitBranches(projectId ?? ''),
        enabled: !!projectId,
        retry: false,
    })

export const gitStashesQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.STASHES(projectId ?? ''),
        queryFn: () => getGitStashes(projectId ?? ''),
        enabled: !!projectId,
        retry: false,
    })

export const useCreateGitBranch = (projectId: ProjectId | null) => useGitMutation(projectId, createGitBranch)

export const useCheckoutGitBranch = (projectId: ProjectId | null) => useGitMutation(projectId, checkoutGitBranch)

export const useDeleteGitBranch = (projectId: ProjectId | null) => useGitMutation(projectId, deleteGitBranch)

export const usePushGitStash = (projectId: ProjectId | null) => useGitMutation(projectId, pushGitStash)

export const useApplyGitStash = (projectId: ProjectId | null) => useGitMutation(projectId, applyGitStash)

export const useDropGitStash = (projectId: ProjectId | null) => useGitMutation(projectId, dropGitStash)

export const useDiscardGitHunk = (projectId: ProjectId | null) => useGitMutation(projectId, discardGitHunk)
