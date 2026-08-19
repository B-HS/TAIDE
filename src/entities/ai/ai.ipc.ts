import { getCurrentWindow } from '@tauri-apps/api/window'
import { commands } from '@shared/api/bindings'
import type { AiCommitMessageRequest, AiInlineCompleteRequest, AiProviderId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getAiTokenStatus = () => unwrapResult(commands.aiTokenStatus())

export const setAiToken = (provider: AiProviderId, token: string) => unwrapResult(commands.aiSetToken(provider, token))

export const clearAiToken = (provider: AiProviderId) => unwrapResult(commands.aiClearToken(provider))

export const listAiModels = (provider: AiProviderId) => unwrapResult(commands.aiListModels(provider))

/**
 * `owner` (this window's OS label) scopes the in-flight request in Rust's `AiRequestStore` — mirrors
 * `entities/lsp/lsp.ipc.ts`'s `spawnLspSession` `owner` precedent. Injected here so callers (the
 * auto-tab inline-completion client, Inline Edit) never have to know about window scoping.
 */
export const completeAiInline = (request: Omit<AiInlineCompleteRequest, 'owner'>) =>
    unwrapResult(commands.aiInlineComplete({ ...request, owner: getCurrentWindow().label }))

export const generateAiCommitMessage = (request: Omit<AiCommitMessageRequest, 'owner'>) =>
    unwrapResult(commands.aiCommitMessage({ ...request, owner: getCurrentWindow().label }))

/**
 * Cancels any in-flight AI request by `requestId` — the Rust-side store (`AiRequestStore`) backs
 * every AI feature's cancellation (auto-tab inline completion, Inline Edit, AI commit messages)
 * behind the single `ai_request_cancel` command. `owner` must match the window that began the
 * request (`AiRequestStore`'s `(owner, requestId)` composite key — R6#20).
 */
export const cancelAiRequest = (requestId: string) => unwrapResult(commands.aiRequestCancel(getCurrentWindow().label, requestId))
