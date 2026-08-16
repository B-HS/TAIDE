import { commands } from '@shared/api/bindings'
import type { AiInlineEditRequest } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const requestAiInlineEdit = (request: AiInlineEditRequest) => unwrapResult(commands.aiInlineEdit(request))
