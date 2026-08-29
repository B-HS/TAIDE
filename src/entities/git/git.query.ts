import { keepPreviousData, queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { DiffMode, OpenedFile, ProjectId } from '@shared/api/bindings'
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
    getGitBlameRange,
    getGitBranches,
    getGitCommitFiles,
    getGitConflictSides,
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

/**
 * The cursor-line blame footer's data source (contract T1 3차 batch 4, F1#17) — `line` is expected to
 * already be debounced by the caller (`use-editor-blame.ts`), not the current, still-moving cursor
 * position, so this factory itself does no debouncing. `placeholderData: keepPreviousData` keeps the
 * previously resolved line's blame visible while a newly debounced line is in flight, matching the
 * pre-query effect this replaces (which only ever overwrote the footer's text once a fetch resolved,
 * never blanked it mid-flight).
 */
export const gitBlameLineQueryOptions = (input: { projectId: ProjectId | null; path: string | null; line: number | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.BLAME_LINE(input.projectId ?? '', input.path ?? '', input.line ?? 0),
        queryFn: () =>
            getGitBlameRange({ projectId: input.projectId ?? '', path: input.path ?? '', from: input.line ?? 0, to: input.line ?? 0 }).then(
                (lines) => lines[0] ?? null,
            ),
        enabled: !!input.projectId && !!input.path && input.line !== null,
        placeholderData: keepPreviousData,
        retry: false,
    })

/**
 * The whole-file blame overlay's data source (`git.toggleBlame`). `lineCount` is deliberately not
 * part of the query key — the pre-query effect this replaces only recomputed `model.getLineCount()`
 * when *itself* re-ran (editor/enabled/project/path changing), never on every keystroke that changes
 * the model's line count, and keeping the key at `QUERY_KEY.GIT.BLAME_OVERLAY(projectId, path)`
 * regardless of `lineCount` preserves that: typing new lines while the overlay is showing doesn't
 * trigger a refetch, only toggling the overlay or switching files does. `staleTime: 0` is what makes
 * "toggling" actually refetch though — without it, re-enabling within the global 60s default would
 * silently reuse whatever `lineCount` was current the *first* time it was enabled, clipping the
 * overlay to that older line count if the file has since grown.
 */
export const gitBlameOverlayQueryOptions = (input: { projectId: ProjectId | null; path: string | null; lineCount: number | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.BLAME_OVERLAY(input.projectId ?? '', input.path ?? ''),
        queryFn: () => getGitBlameRange({ projectId: input.projectId ?? '', path: input.path ?? '', from: 1, to: input.lineCount ?? 1 }),
        enabled: !!input.projectId && !!input.path && input.lineCount !== null,
        staleTime: 0,
        retry: false,
    })

/**
 * The conflict-compare dialog's data source — fetched on demand (the "Compare" button in
 * `ConflictResolutionDialog`), not reactively, so the caller gates `enabled` on a request flag rather
 * than this factory fetching eagerly whenever a path happens to be conflicted.
 */
export const gitConflictSidesQueryOptions = (input: { projectId: ProjectId | null; path: string | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.CONFLICT_SIDES(input.projectId ?? '', input.path ?? ''),
        queryFn: () => getGitConflictSides({ projectId: input.projectId ?? '', path: input.path ?? '' }),
        enabled: !!input.projectId && !!input.path,
        retry: false,
    })

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

export const gitDiffFileQueryOptions = (input: { projectId: ProjectId | null; path: string | null; mode: DiffMode; beforePath?: string | null }) =>
    queryOptions({
        queryKey: QUERY_KEY.GIT.DIFF(input.projectId ?? '', input.path ?? '', input.mode, input.beforePath ?? null),
        queryFn: () =>
            getGitDiffFile({ projectId: input.projectId ?? '', path: input.path ?? '', mode: input.mode, beforePath: input.beforePath ?? null }),
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

/**
 * Unlike every other mutation funneled through {@link useGitMutation}, resolving a conflict also
 * rewrites the file's on-disk content directly — `git_resolve_conflict` writes `content` to `path` as
 * part of resolving it — so on top of the coarse git-scope invalidation every mutation gets, this one
 * also invalidates the `FILE` domain's cache for that exact file: `FILE.CONTENT` (so the file query
 * `EditorPane` itself reads catches up to what's now on disk) and `FILE.MIRRORS` (the backend already
 * discards any hot-exit mirror for `path` as part of the same write, so the stale cached entry must
 * go too). This mirrors `entities/file/file.query.ts`'s `useRenameEntry`/`useCopyEntry`/
 * `useDeleteEntry` doing the reverse cross-domain invalidation (a `FILE` mutation invalidating
 * `GIT.PROJECT`) — this codebase already accepts one entity's mutation reaching into a sibling
 * domain's query keys when its own side effect spans both (contract F3#4).
 */
export const useResolveGitConflict = (projectId: ProjectId | null) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: resolveGitConflict,
        /**
         * Same disk-write cache patch as `useSaveFile` (`entities/file/file.query.ts` — see its
         * doc comment for the stale-adoption clobber this closes,
         * `2026-08-27-d43-save-stale-sync-clobber-contract.md` §0): this mutation wrote `content`
         * to `path`, so `FILE.CONTENT` must say so before any dirty→false settle re-render can
         * re-adopt the pre-write cache entry.
         */
        onSuccess: (_, { path, content }) => {
            queryClient.setQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT(path), (existing) => (existing ? { ...existing, content } : existing))
            void queryClient.invalidateQueries({
                queryKey: QUERY_KEY.GIT.PROJECT(projectId ?? ''),
                predicate: (query) => isGitQueryScopeMutable(query.queryKey),
            })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId ?? '') })
        },
    })
}

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
