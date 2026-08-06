import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { openFile, saveFile } from '@entities/file/file.ipc'

export const fileQueryOptions = (path: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.FILE.CONTENT(path ?? ''),
        queryFn: () => openFile(path ?? ''),
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
