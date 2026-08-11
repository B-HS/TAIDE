import { useMutation } from '@tanstack/react-query'
import { extractVsixThemes } from '@entities/vsix/vsix.ipc'

export const useExtractVsixThemes = () => useMutation({ mutationFn: extractVsixThemes })
