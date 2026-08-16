import { useMutation, useQueryClient } from '@tanstack/react-query'
import { refetchAndApplyPluginList } from '@entities/plugin/plugin.query'
import { extractVsixThemes, importVsixPlugin } from '@entities/vsix/vsix.ipc'

export const useExtractVsixThemes = () => useMutation({ mutationFn: extractVsixThemes })

export const useImportVsixPlugin = () => {
    const queryClient = useQueryClient()
    return useMutation({ mutationFn: importVsixPlugin, onSuccess: () => refetchAndApplyPluginList(queryClient) })
}
