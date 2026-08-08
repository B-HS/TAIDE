import { toast } from 'sonner'
import type { KeymapActionId } from '@shared/lib/keymap'
import { buildImeDebugReport } from '@shared/lib/ime-debug'
import { i18next } from '@shared/i18n/i18n'
import { requestOpenSearchPanel } from '@shared/lib/search-panel-bridge'

export type CommandContext = {
    activeProjectId: string | null
    openSettingsTab: () => void
    openTerminalTab: () => void
    reopenClosedTab: () => void
    switchToFileSearchMode: () => void
}

export type AppCommand = {
    id: string
    titleKey: string
    categoryKey?: string
    keymapId?: KeymapActionId
    run: (context: CommandContext) => void | Promise<void>
    isEnabled?: (context: CommandContext) => boolean
}

export type PaletteMode = 'commands' | 'files'

export const COMMAND_MODE_PREFIX = '>'

export type ParsedPaletteQuery = {
    mode: PaletteMode
    searchTerm: string
}

export const parsePaletteQuery = (rawQuery: string): ParsedPaletteQuery =>
    rawQuery.startsWith(COMMAND_MODE_PREFIX)
        ? { mode: 'commands', searchTerm: rawQuery.slice(COMMAND_MODE_PREFIX.length).trimStart() }
        : { mode: 'files', searchTerm: rawQuery }

export const buildCommandModeQuery = (searchTerm: string = '') => `${COMMAND_MODE_PREFIX}${searchTerm}`

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

const notImplementedRun = () => {}

const alwaysDisabled = () => false

export const DEFAULT_COMMANDS: AppCommand[] = [
    { id: 'window.reload', titleKey: 'app.reloadWindow', run: () => window.location.reload() },
    { id: 'settings.open', titleKey: 'settings.title', run: (context) => context.openSettingsTab() },
    { id: 'terminal.new', titleKey: 'keymap.newTerminal', keymapId: 'new-terminal', run: (context) => context.openTerminalTab() },
    { id: 'tab.reopenClosed', titleKey: 'keymap.reopenClosedTab', keymapId: 'reopen-closed-tab', run: (context) => context.reopenClosedTab() },
    { id: 'file.quickOpen', titleKey: 'keymap.quickOpen', keymapId: 'quick-open', run: (context) => context.switchToFileSearchMode() },
    { id: 'tab.close', titleKey: 'keymap.closeTab', keymapId: 'close-tab', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'view.toggleSidebar', titleKey: 'keymap.toggleSidebar', keymapId: 'toggle-sidebar', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'editor.find', titleKey: 'keymap.find', keymapId: 'find', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'search.find', titleKey: 'keymap.search', keymapId: 'search', run: () => requestOpenSearchPanel() },
    { id: 'search.replace', titleKey: 'keymap.searchReplace', keymapId: 'search-replace', run: () => requestOpenSearchPanel({ openReplace: true }) },
    { id: 'view.explorer', titleKey: 'keymap.explorer', keymapId: 'explorer', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'view.git', titleKey: 'git.title', keymapId: 'git', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'editor.split', titleKey: 'keymap.split', keymapId: 'split', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'tab.cycleNext', titleKey: 'keymap.tabCycleNext', keymapId: 'tab-cycle-next', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'tab.cyclePrev', titleKey: 'keymap.tabCyclePrev', keymapId: 'tab-cycle-prev', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'editor.save', titleKey: 'keymap.save', keymapId: 'save', run: notImplementedRun, isEnabled: alwaysDisabled },
    { id: 'view.toggleTerminal', titleKey: 'keymap.toggleTerminal', keymapId: 'toggle-terminal', run: notImplementedRun, isEnabled: alwaysDisabled },
    {
        id: 'terminal.copyImeDebug',
        titleKey: 'terminal.copyImeDebugLog',
        run: async () => {
            await navigator.clipboard.writeText(buildImeDebugReport())
            toast.success(i18next.t('terminal.imeDebugCopied'))
        },
    },
]
