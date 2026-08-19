import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { copyEntry, createEntry, deleteEntry, listMirrors, listUntitledMirrors, openFile, renameEntry, saveFile } from '@entities/file/file.ipc'
import { readFileRaw } from '@entities/file/file.raw'

export const fileQueryOptions = (path: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.CONTENT(path ?? ''),
        queryFn: () => openFile(path ?? ''),
        enabled: !!path,
        staleTime: Infinity,
    })

/**
 * Restorable hot-exit mirrors for the project's file tabs, fetched once when a project becomes
 * active (`staleTime: Infinity` — invalidated explicitly on save/clear/prune rather than refetched
 * on a timer) and shared by every editor pane through the query cache.
 */
export const fileMirrorsQueryOptions = (projectId: ProjectId) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.MIRRORS(projectId),
        queryFn: () => listMirrors(projectId),
        staleTime: Infinity,
    })

export const untitledMirrorsQueryOptions = (projectId: ProjectId) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId),
        queryFn: () => listUntitledMirrors(projectId),
        staleTime: Infinity,
    })

export const fileRawQueryOptions = (path: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.RAW(path ?? ''),
        queryFn: () => readFileRaw(path ?? ''),
        enabled: !!path,
        staleTime: Infinity,
    })

/**
 * Saving a file changes its git status and settles its hot-exit mirror, so callers that know the
 * owning project pass `projectId` to have those scopes invalidated here (the entities layer owns
 * invalidation — see `useRenameEntry` for the same pattern) instead of each widget re-invalidating
 * by hand. Callers without a project context (`ide-sync-provider`) omit it and keep the
 * path-scoped `FILE.CONTENT` refresh only.
 */
export const useSaveFile = (projectId?: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: saveFile,
        onSuccess: (_, { path }) => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
            if (!projectId) return
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
        },
    })
}

export const useCreateEntry = () => useMutation({ mutationFn: createEntry })

export const useRenameEntry = (projectId: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: renameEntry,
        onSuccess: (_, { from, to }) => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(from) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(to) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
        },
    })
}

export const useCopyEntry = (projectId: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: copyEntry,
        onSuccess: (_, { to }) => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(to) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
        },
    })
}

export const useDeleteEntry = (projectId: ProjectId) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: deleteEntry,
        onSuccess: (_, path) => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
        },
    })
}
