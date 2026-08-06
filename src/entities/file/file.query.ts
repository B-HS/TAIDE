import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createEntry, openFile, saveFile } from '@entities/file/file.ipc'
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
