import { describe, expect, test } from 'bun:test'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { isCommandRunnable } from '@shared/lib/command-registry'
import { TASK_COMMANDS } from '@entities/task/task.commands'

const dummyContext: CommandContext = {
    activeProjectId: null,
    activeEditorActionIds: null,
    openSettingsTab: () => {},
    openSettingsFile: () => {},
    openTerminalTab: () => {},
    openWelcomeTab: () => {},
    reopenClosedTab: () => {},
    switchToFileSearchMode: () => {},
}

describe('TASK_COMMANDS', () => {
    test('활성 프로젝트가 없으면 runTask 커맨드는 비활성이다', () => {
        const runTask = TASK_COMMANDS.find((command) => command.id === 'task.runTask')
        expect(runTask).toBeDefined()
        expect(isCommandRunnable(runTask as AppCommand, dummyContext)).toBe(false)
    })

    test('활성 프로젝트가 있으면 runTask 커맨드는 활성이다', () => {
        const context: CommandContext = { ...dummyContext, activeProjectId: 'p1' }
        const runTask = TASK_COMMANDS.find((command) => command.id === 'task.runTask')
        expect(runTask).toBeDefined()
        expect(isCommandRunnable(runTask as AppCommand, context)).toBe(true)
    })
})
