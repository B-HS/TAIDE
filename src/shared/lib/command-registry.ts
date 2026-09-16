import type { TFunction } from 'i18next'
import { toast } from 'sonner'
import type { KeymapActionId } from '@shared/lib/keymap/keymap'
import { describeIpcError } from '@shared/lib/ipc-error-message'

export type CommandContext = {
    activeProjectId: string | null
    /**
     * Which shell slot the main window currently has focus in, or `null` outside a split shell
     * (an auxiliary window, a window with a single slot). Read-only — contract §0.1 U-5 keeps slot
     * addressing out of the ~12 publishing call sites by gating the *subscribers* instead, so this
     * exists for commands that need to name the slot they act on rather than to route a broadcast.
     */
    focusedShellSlotId: string | null
    activeEditorActionIds: Set<string> | null
    openSettingsTab: () => void
    openSettingsFile: () => void
    openTerminalTab: () => void
    openWelcomeTab: () => void
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

/**
 * Dispatches one command and reports whatever it fails with, instead of dropping it.
 *
 * `run` is declared `void | Promise<void>` and the palette's two dispatch sites (the keydown
 * capture and the list selection) both discarded the result with `void`, so an async command that
 * rejected became an unhandled rejection — a line in the file log through the global forwarder
 * (`error-log-forwarding.ts`) and nothing at all on screen, with the command's own success toast
 * skipped. Centralising it here also means the next async command inherits the handling rather than
 * re-opening the same hole.
 *
 * The `try`/`catch` wraps the call itself rather than chaining `.catch()` onto its result, because a
 * synchronous `run` throws *before* there is a promise to attach a handler to — the same reason
 * `monaco/on-save-cleanup.ts` guards `IEditorAction.run` this way. `Promise.resolve(...)` around the
 * call would not help either: the throw escapes while its argument is being evaluated.
 */
export const runCommandSafely = (command: AppCommand, context: CommandContext) => {
    const report = (error: unknown) => toast.error(describeIpcError(error))
    try {
        const result = command.run(context)
        if (result instanceof Promise) void result.catch(report)
    } catch (error) {
        report(error)
    }
}

export const formatCategorizedLabel = (t: TFunction, categoryKey: string | null | undefined, titleKey: string, titleDefaultValue?: string) => {
    const title = titleDefaultValue ? t(titleKey, { defaultValue: titleDefaultValue }) : t(titleKey)
    return categoryKey ? `${t(categoryKey)}: ${title}` : title
}
