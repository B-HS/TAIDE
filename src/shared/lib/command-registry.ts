import type { TFunction } from 'i18next'
import type { KeymapActionId } from '@shared/lib/keymap/keymap'

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

const snapshotListeners = new Set<() => void>()

let commandsSnapshot: AppCommand[] = []

/**
 * Rebuilds the array {@link listRegisteredCommands} hands out and notifies
 * {@link subscribeRegisteredCommands} subscribers. Every registry write goes through here, so the
 * snapshot's identity changes if and only if the registry's contents did — which is what lets
 * `command-palette.tsx` read it through `useSyncExternalStore` and lets React Compiler memoize the
 * (per-render, whole-catalog) `buildKeybindingRows`/`fuzzyFilter` work built on top of it. The old
 * `Array.from(...)` per call returned a fresh array on every render and defeated that entirely
 * (audit §1-14).
 */
const publishCommandsSnapshot = () => {
    commandsSnapshot = Array.from(commandRegistry.values())
    snapshotListeners.forEach((listener) => listener())
}

export const registerCommand = (command: AppCommand) => {
    commandRegistry.set(command.id, command)
    publishCommandsSnapshot()
}

export const registerCommands = (commands: AppCommand[]) => {
    for (const command of commands) commandRegistry.set(command.id, command)
    publishCommandsSnapshot()
}

export const unregisterCommand = (id: string) => {
    commandRegistry.delete(id)
    publishCommandsSnapshot()
}

export const clearCommandRegistry = () => {
    commandRegistry.clear()
    publishCommandsSnapshot()
}

export const getRegisteredCommand = (id: string) => commandRegistry.get(id) ?? null

/**
 * The shared snapshot array, not a copy — callers must treat it as read-only (every consumer today
 * only filters or maps over it).
 */
export const listRegisteredCommands = () => commandsSnapshot

export const subscribeRegisteredCommands = (listener: () => void) => {
    snapshotListeners.add(listener)
    return () => {
        snapshotListeners.delete(listener)
    }
}

export const isCommandRunnable = (command: AppCommand, context: CommandContext) => (command.isEnabled ? command.isEnabled(context) : true)

export const formatCategorizedLabel = (t: TFunction, categoryKey: string | null | undefined, titleKey: string, titleDefaultValue?: string) => {
    const title = titleDefaultValue ? t(titleKey, { defaultValue: titleDefaultValue }) : t(titleKey)
    return categoryKey ? `${t(categoryKey)}: ${title}` : title
}
