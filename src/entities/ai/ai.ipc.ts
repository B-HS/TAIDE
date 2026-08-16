import { commands } from '@shared/api/bindings'
import type { AiCommitMessageRequest, AiInlineCompleteRequest, AiProviderId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getAiTokenStatus = () => unwrapResult(commands.aiTokenStatus())

export const setAiToken = (provider: AiProviderId, token: string) => unwrapResult(commands.aiSetToken(provider, token))

export const clearAiToken = (provider: AiProviderId) => unwrapResult(commands.aiClearToken(provider))

export const listAiModels = (provider: AiProviderId) => unwrapResult(commands.aiListModels(provider))

export const completeAiInline = (request: AiInlineCompleteRequest) => unwrapResult(commands.aiInlineComplete(request))

export const generateAiCommitMessage = (request: AiCommitMessageRequest) => unwrapResult(commands.aiCommitMessage(request))

/**
 * Cancels any in-flight AI request by `requestId` — the Rust-side store (`AiRequestStore`) backs
 * every AI feature's cancellation (auto-tab inline completion, Inline Edit, AI commit messages)
 * behind the single `ai_request_cancel` command.
 */
export const cancelAiRequest = (requestId: string) => unwrapResult(commands.aiRequestCancel(requestId))
