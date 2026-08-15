import { queryOptions } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { detectTasks } from '@entities/task/task.ipc'

export const tasksQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.TASK.LIST(projectId ?? ''),
        queryFn: () => detectTasks(projectId ?? ''),
        enabled: !!projectId,
    })
