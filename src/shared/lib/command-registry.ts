import type { TFunction } from 'i18next'
import { toast } from 'sonner'
import type { KeymapActionId } from '@shared/lib/keymap'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { requestShowExplorerView, requestToggleExplorerSidebar } from '@shared/lib/explorer-panel-bridge'
import { buildImeDebugReport } from '@shared/lib/ime-debug'
import { i18next } from '@shared/i18n/i18n'
import { requestOpenKeybindingsEditor } from '@shared/lib/keybindings-bridge'
import { requestOpenSearchPanel } from '@shared/lib/search-panel-bridge'

export const KEYMAP_CATEGORY = {
    APP: 'keymap.category.app',
    VIEW: 'keymap.category.view',
    EDITOR: 'keymap.category.editor',
    TAB: 'keymap.category.tab',
    TERMINAL: 'keymap.category.terminal',
    SEARCH: 'keymap.category.search',
    FILE: 'keymap.category.file',
    SYNC: 'keymap.category.sync',
    WINDOW: 'keymap.category.window',
} as const

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

export const formatCategorizedLabel = (t: TFunction, categoryKey: string | null | undefined, titleKey: string) =>
    categoryKey ? `${t(categoryKey)}: ${t(titleKey)}` : t(titleKey)

const notImplementedRun = () => {}

const alwaysDisabled = () => false

export const DEFAULT_COMMANDS: AppCommand[] = [
    { id: 'window.reload', titleKey: 'app.reloadWindow', categoryKey: KEYMAP_CATEGORY.WINDOW, run: () => window.location.reload() },
    { id: 'settings.open', titleKey: 'settings.title', categoryKey: KEYMAP_CATEGORY.APP, run: (context) => context.openSettingsTab() },
    {
        id: 'keybindings.open',
        titleKey: 'settings.keymapOpenEditor',
        categoryKey: KEYMAP_CATEGORY.APP,
        run: () => requestOpenKeybindingsEditor(),
    },
    {
        id: 'terminal.new',
        titleKey: 'keymap.newTerminal',
        categoryKey: KEYMAP_CATEGORY.TERMINAL,
        keymapId: 'new-terminal',
        run: (context) => context.openTerminalTab(),
    },
    {
        id: 'tab.reopenClosed',
        titleKey: 'keymap.reopenClosedTab',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'reopen-closed-tab',
        run: (context) => context.reopenClosedTab(),
    },
    {
        id: 'file.quickOpen',
        titleKey: 'keymap.quickOpen',
        categoryKey: KEYMAP_CATEGORY.FILE,
        keymapId: 'quick-open',
        run: (context) => context.switchToFileSearchMode(),
    },
    {
        id: 'tab.close',
        titleKey: 'keymap.closeTab',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'close-tab',
        run: notImplementedRun,
        isEnabled: alwaysDisabled,
    },
    {
        id: 'view.toggleSidebar',
        titleKey: 'keymap.toggleSidebar',
        categoryKey: KEYMAP_CATEGORY.VIEW,
        keymapId: 'toggle-sidebar',
        run: () => requestToggleExplorerSidebar(),
    },
    {
        id: 'editor.find',
        titleKey: 'keymap.find',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'find',
        run: notImplementedRun,
        isEnabled: alwaysDisabled,
    },
    { id: 'search.find', titleKey: 'keymap.search', categoryKey: KEYMAP_CATEGORY.SEARCH, keymapId: 'search', run: () => requestOpenSearchPanel() },
    {
        id: 'search.replace',
        titleKey: 'keymap.searchReplace',
        categoryKey: KEYMAP_CATEGORY.SEARCH,
        keymapId: 'search-replace',
        run: () => requestOpenSearchPanel({ openReplace: true }),
    },
    {
        id: 'view.explorer',
        titleKey: 'keymap.explorer',
        categoryKey: KEYMAP_CATEGORY.VIEW,
        keymapId: 'explorer',
        run: () => requestShowExplorerView('files'),
    },
    { id: 'view.git', titleKey: 'git.title', categoryKey: KEYMAP_CATEGORY.VIEW, keymapId: 'git', run: () => requestShowExplorerView('git') },
    {
        id: 'editor.split',
        titleKey: 'keymap.split',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'split',
        run: () => requestEditorPaneCommand({ type: 'split' }),
    },
    {
        id: 'tab.cycleNext',
        titleKey: 'keymap.tabCycleNext',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'tab-cycle-next',
        run: () => requestEditorPaneCommand({ type: 'cycle-tab', direction: 'next' }),
    },
    {
        id: 'tab.cyclePrev',
        titleKey: 'keymap.tabCyclePrev',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'tab-cycle-prev',
        run: () => requestEditorPaneCommand({ type: 'cycle-tab', direction: 'prev' }),
    },
    {
        id: 'editor.save',
        titleKey: 'keymap.save',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'save',
        run: () => requestEditorPaneCommand({ type: 'save-active-tab' }),
    },
    {
        id: 'view.toggleTerminal',
        titleKey: 'keymap.toggleTerminal',
        categoryKey: KEYMAP_CATEGORY.VIEW,
        keymapId: 'toggle-terminal',
        run: () => requestEditorPaneCommand({ type: 'toggle-terminal' }),
    },
    {
        id: 'terminal.copyImeDebug',
        titleKey: 'terminal.copyImeDebugLog',
        categoryKey: KEYMAP_CATEGORY.TERMINAL,
        run: async () => {
            await navigator.clipboard.writeText(buildImeDebugReport())
            toast.success(i18next.t('terminal.imeDebugCopied'))
        },
    },
]
