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
        /**
         * Same disk-write cache patch as `useSaveFile` (`entities/file/file.query.ts` — see its doc
         * comment for the stale-adoption clobber this closes,
         * `2026-08-27-d43-save-stale-sync-clobber-contract.md` §0): `APP_FILE.CONTENT` holds the raw
         * string, so the written `content` replaces it wholesale before the confirming refetch.
         */
        onSuccess: (_, { target, content }) => {
            queryClient.setQueryData(QUERY_KEY.APP_FILE.CONTENT(target), content)
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.APP_FILE.CONTENT(target) })
        },
    })
}
