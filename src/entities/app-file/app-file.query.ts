import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { AppFileTarget } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { readAppFile, writeAppFile } from '@entities/app-file/app-file.ipc'

export const appFileQueryOptions = (target: AppFileTarget) =>
    queryOptions({ queryKey: QUERY_KEY.APP_FILE.CONTENT(target), queryFn: () => readAppFile(target), staleTime: Infinity })

export const useWriteAppFile = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: writeAppFile,
        onSuccess: (_, { target }) => queryClient.invalidateQueries({ queryKey: QUERY_KEY.APP_FILE.CONTENT(target) }),
    })
}
