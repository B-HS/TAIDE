import { getCurrentWindow } from '@tauri-apps/api/window'
import { commands } from '@shared/api/bindings'
import type { AiInlineEditRequest } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

/** `owner` (this window's OS label) is injected here — see `entities/ai/ai.ipc.ts`'s `completeAiInline` precedent. */
export const requestAiInlineEdit = (request: Omit<AiInlineEditRequest, 'owner'>) =>
    unwrapResult(commands.aiInlineEdit({ ...request, owner: getCurrentWindow().label }))
