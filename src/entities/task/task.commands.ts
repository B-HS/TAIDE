import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap/keymap-category'
import { requestOpenTaskRunner } from '@shared/lib/bridge/task-runner-bridge'

const isProjectActive = (context: CommandContext) => context.activeProjectId !== null

export const TASK_COMMANDS: AppCommand[] = [
    {
        id: 'task.runTask',
        titleKey: 'task.runTask',
        categoryKey: KEYMAP_CATEGORY.TERMINAL,
        run: () => requestOpenTaskRunner(),
        isEnabled: isProjectActive,
    },
]
