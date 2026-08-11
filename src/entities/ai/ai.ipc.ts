import { commands } from '@shared/api/bindings'
import type { AiInlineCompleteRequest, AiProviderId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getAiTokenStatus = () => unwrapResult(commands.aiTokenStatus())

export const setAiToken = (provider: AiProviderId, token: string) => unwrapResult(commands.aiSetToken(provider, token))

export const clearAiToken = (provider: AiProviderId) => unwrapResult(commands.aiClearToken(provider))

export const listAiModels = (provider: AiProviderId) => unwrapResult(commands.aiListModels(provider))

export const completeAiInline = (request: AiInlineCompleteRequest) => unwrapResult(commands.aiInlineComplete(request))

export const cancelAiInline = (requestId: string) => unwrapResult(commands.aiInlineCancel(requestId))
