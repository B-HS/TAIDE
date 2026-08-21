import type { TFunction } from 'i18next'
import type { KeymapActionId } from '@shared/lib/keymap'

export type CommandContext = {
    activeProjectId: string | null
    activeEditorActionIds: Set<string> | null
    openSettingsTab: () => void
    openSettingsFile: () => void
    openTerminalTab: () => void
    reopenClosedTab: () => void
    switchToFileSearchMode: () => void
}

export type AppCommand = {
    id: string
    titleKey: string
    categoryKey?: string
    keymapId?: KeymapActionId
    titleDefaultValue?: string
    run: (context: CommandContext) => void | Promise<void>
    isEnabled?: (context: CommandContext) => boolean
}

const commandRegistry = new Map<string, AppCommand>()

export const registerCommand = (command: AppCommand) => {
    commandRegistry.set(command.id, command)
}

export const registerCommands = (commands: AppCommand[]) => {
    for (const command of commands) registerCommand(command)
}

export const unregisterCommand = (id: string) => {
    commandRegistry.delete(id)
}

export const clearCommandRegistry = () => {
    commandRegistry.clear()
}

export const getRegisteredCommand = (id: string) => commandRegistry.get(id) ?? null

export const listRegisteredCommands = () => Array.from(commandRegistry.values())

export const isCommandRunnable = (command: AppCommand, context: CommandContext) => (command.isEnabled ? command.isEnabled(context) : true)

export const formatCategorizedLabel = (t: TFunction, categoryKey: string | null | undefined, titleKey: string, titleDefaultValue?: string) => {
    const title = titleDefaultValue ? t(titleKey, { defaultValue: titleDefaultValue }) : t(titleKey)
    return categoryKey ? `${t(categoryKey)}: ${title}` : title
}
