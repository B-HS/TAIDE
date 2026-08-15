import type { ProjectId } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const detectTasks = (projectId: ProjectId) => unwrapResult(commands.detectTasks(projectId))
