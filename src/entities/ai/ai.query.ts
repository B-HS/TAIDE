import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { AiProviderId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { clearAiToken, getAiTokenStatus, listAiModels, setAiToken } from '@entities/ai/ai.ipc'

const DEFAULT_MODELS_QUERY_PROVIDER: AiProviderId = 'ollamaCloud'

export const aiTokenStatusQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.AI.TOKEN_STATUS, queryFn: getAiTokenStatus })

export const aiModelsQueryOptions = (provider: AiProviderId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.MODELS(provider ?? DEFAULT_MODELS_QUERY_PROVIDER),
        queryFn: () => listAiModels(provider ?? DEFAULT_MODELS_QUERY_PROVIDER),
        enabled: provider !== null,
        staleTime: Infinity,
    })

export const useSetAiToken = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: ({ provider, token }: { provider: AiProviderId; token: string }) => setAiToken(provider, token),
        onSuccess: (_data, variables) => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.TOKEN_STATUS })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.MODELS(variables.provider) })
        },
    })
}

export const useClearAiToken = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (provider: AiProviderId) => clearAiToken(provider),
        onSuccess: (_data, provider) => {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.TOKEN_STATUS })
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.MODELS(provider) })
        },
    })
}
