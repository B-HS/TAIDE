import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { DiffMode, ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { cancelAiRequest, generateAiCommitMessage } from '@entities/ai/ai.ipc'
import {
    applyGitStash,
    checkoutGitBranch,
    checkoutRemoteGitBranch,
    commitGit,
    createGitBranch,
    createGitTag,
    deleteGitBranch,
    deleteGitTag,
    discardGitHunk,
    discardGitPaths,
    dropGitStash,
    getGitBranches,
    getGitCommitFiles,
    getGitDiffStagedText,
    getGitStashes,
    getGitTags,
    pushGitStash,
    getGitCurrentUser,
    getGitDiffFile,
    getGitFileLog,
    getGitGutter,
    getGitLog,
    getGitRemotes,
    getGitShowFile,
    getGitStatus,
    initGitRepository,
    pullGit,
    pushGit,
    resolveGitConflict,
    revertGitCommit,
    stageGitHunk,
    stageGitLines,
    stageGitPaths,
    unstageGitHunk,
    unstageGitLines,
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

/**
 * Whether a git query outside the rev-immutable scopes (`QUERY_KEY.GIT.REV_IMMUTABLE_SCOPES` —
 * commit file lists, commit blob diffs) should be swept up by a mutation's coarse project-prefix
 * invalidation. Those scopes are keyed by an immutable `rev` rather than live working-tree/index
 * state, so their `staleTime: Infinity` queries already stay correct for as long as the panel
 * showing them stays open — invalidating them on every unrelated stage/unstage/stash/etc. would
 * just discard that cache for no reason.
 */
export const isGitQueryScopeMutable = (queryKey: readonly unknown[]) =>
    !(QUERY_KEY.GIT.REV_IMMUTABLE_SCOPES as readonly unknown[]).includes(queryKey[2])

/**
 * `queryKey` and `predicate` combine with AND (TanStack Query's `matchQuery`), so scoping the
 * predicate to {@link isGitQueryScopeMutable} keeps the coarse `['git', projectId, ...]`-prefix
 * invalidation everything else relies on.
 */
const useGitMutation = <TVariables, TResult>(projectId: ProjectId | null, mutationFn: (variables: TVariables) => Promise<TResult>) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn,
        onSuccess: () =>
            queryClient.invalidateQueries({
                queryKey: QUERY_KEY.GIT.PROJECT(projectId ?? ''),
                predicate: (query) => isGitQueryScopeMutable(query.queryKey),
            }),
    })
}

export const useInitGitRepository = (projectId: ProjectId | null) => useGitMutation(projectId, initGitRepository)

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

export const gitTagsQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.TAGS(projectId ?? ''),
        queryFn: () => getGitTags(projectId ?? ''),
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

export const useResolveGitConflict = (projectId: ProjectId | null) => useGitMutation(projectId, resolveGitConflict)

export const useStageGitHunk = (projectId: ProjectId | null) => useGitMutation(projectId, stageGitHunk)

export const useUnstageGitHunk = (projectId: ProjectId | null) => useGitMutation(projectId, unstageGitHunk)

export const useStageGitLines = (projectId: ProjectId | null) => useGitMutation(projectId, stageGitLines)

export const useUnstageGitLines = (projectId: ProjectId | null) => useGitMutation(projectId, unstageGitLines)

export const useRevertGitCommit = (projectId: ProjectId | null) => useGitMutation(projectId, revertGitCommit)

export const useCreateGitTag = (projectId: ProjectId | null) => useGitMutation(projectId, createGitTag)

export const useDeleteGitTag = (projectId: ProjectId | null) => useGitMutation(projectId, deleteGitTag)

export const useCheckoutRemoteGitBranch = (projectId: ProjectId | null) => useGitMutation(projectId, checkoutRemoteGitBranch)

export const gitCommitFilesQueryOptions = (input: { projectId: ProjectId | null; rev: string | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.COMMIT_FILES(input.projectId ?? '', input.rev ?? ''),
        queryFn: () => getGitCommitFiles({ projectId: input.projectId ?? '', rev: input.rev ?? '' }),
        enabled: !!input.projectId && !!input.rev,
        staleTime: Infinity,
        retry: false,
    })

export const gitFileLogQueryOptions = (input: { projectId: ProjectId | null; path: string | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.FILE_LOG(input.projectId ?? '', input.path ?? ''),
        queryFn: () => getGitFileLog({ projectId: input.projectId ?? '', path: input.path ?? '', skip: 0, take: LOG_PAGE_SIZE }),
        enabled: !!input.projectId && !!input.path,
        retry: false,
    })

export const gitShowFileQueryOptions = (input: { projectId: ProjectId | null; rev: string | null; path: string | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.SHOW(input.projectId ?? '', input.rev ?? '', input.path ?? ''),
        queryFn: () => getGitShowFile({ projectId: input.projectId ?? '', rev: input.rev ?? '', path: input.path ?? '' }),
        enabled: !!input.projectId && !!input.rev && !!input.path,
        staleTime: Infinity,
        retry: false,
    })

/**
 * Fetches the staged diff and asks the AI provider for a commit message summarizing it, wrapped in
 * `useMutation` instead of a widget hand-rolling the two raw IPC calls itself (contract F4#5). The
 * cancel-vs-generate toggle, the superseded-request guard, and the toast feedback stay in the
 * widget (`GitPanelContainer`) — that's UI-lifecycle orchestration query.md allows a widget to keep,
 * not server state this hook could own on its behalf.
 */
export const useGenerateAiCommitMessage = (projectId: ProjectId | null) =>
    useMutation({
        mutationFn: async (input: { requestId: string; recentCommitsSummary: string }) => {
            const diff = await getGitDiffStagedText(projectId ?? '')
            const response = await generateAiCommitMessage({
                requestId: input.requestId,
                provider: null,
                model: null,
                diffText: diff.diffText,
                recentCommits: input.recentCommitsSummary,
            })
            return { diff, response }
        },
    })

export const useCancelCommitMessageGeneration = () => useMutation({ mutationFn: cancelAiRequest })
