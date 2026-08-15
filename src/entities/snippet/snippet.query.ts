import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { deleteSnippetFile, listSnippetFiles, saveSnippetFile } from '@entities/snippet/snippet.ipc'

export const snippetListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.SNIPPET.LIST, queryFn: listSnippetFiles })

export const useSnippetList = () => useQuery(snippetListQueryOptions())

export const useSaveSnippet = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: saveSnippetFile,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.SNIPPET.ALL }),
    })
}

export const useDeleteSnippet = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: deleteSnippetFile,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.SNIPPET.ALL }),
    })
}
