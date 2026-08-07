import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { copyEntry, createEntry, deleteEntry, openFile, renameEntry, saveFile } from '@entities/file/file.ipc'
import { readFileRaw } from '@entities/file/file.raw'

export const fileQueryOptions = (path: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.CONTENT(path ?? ''),
        queryFn: () => openFile(path ?? ''),
        enabled: !!path,
        staleTime: Infinity,
    })

export const fileRawQueryOptions = (path: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.RAW(path ?? ''),
        queryFn: () => readFileRaw(path ?? ''),
        enabled: !!path,
        staleTime: Infinity,
    })

export const useSaveFile = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: saveFile,
        onSuccess: (_, { path }) => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) }),
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
