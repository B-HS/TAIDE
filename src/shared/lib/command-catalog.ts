import { toast } from 'sonner'
import type { AppCommand } from '@shared/lib/command-registry'
import { requestEditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'
import { requestShowExplorerView, requestToggleExplorerSidebar } from '@shared/lib/bridge/explorer-panel-bridge'
import { buildImeDebugReport, isImeDebugEnabled } from '@shared/lib/ime-debug'
import { i18next } from '@shared/i18n/i18n'
import { requestOpenKeybindingsEditor } from '@shared/lib/keymap/keybindings-bridge'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap/keymap-category'
import { isPerfEnabled, printPerfReport } from '@shared/lib/perf-mark'
import { requestOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { getWindowContext } from '@shared/lib/window-context'
import { requestToggleZenMode } from '@shared/lib/bridge/zen-mode-bridge'

const notImplementedRun = () => {}

const alwaysDisabled = () => false

export const DEFAULT_COMMANDS: AppCommand[] = [
    { id: 'window.reload', titleKey: 'app.reloadWindow', categoryKey: KEYMAP_CATEGORY.WINDOW, run: () => window.location.reload() },
    { id: 'settings.open', titleKey: 'settings.title', categoryKey: KEYMAP_CATEGORY.APP, run: (context) => context.openSettingsTab() },
    {
        id: 'app.openSettingsFile',
        titleKey: 'app.openSettingsFile',
        categoryKey: KEYMAP_CATEGORY.APP,
        run: (context) => context.openSettingsFile(),
    },
    {
        id: 'keybindings.open',
        titleKey: 'settings.keymapOpenEditor',
        categoryKey: KEYMAP_CATEGORY.APP,
        keymapId: 'open-keybindings-editor',
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
    { id: 'view.welcome', titleKey: 'app.welcome', categoryKey: KEYMAP_CATEGORY.VIEW, run: (context) => context.openWelcomeTab() },
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
        isEnabled: isImeDebugEnabled,
    },
    {
        id: 'app.showPerfSnapshot',
        titleKey: 'app.showPerfSnapshot',
        /**
         * Carries its own English label instead of a locale key, the way the monaco action mirrors
         * do (`monaco-action-commands.ts`): this is a developer instrument that only appears while
         * `TAIDE_PERF` instrumentation is on, and adding three catalog entries for a string no end
         * user can reach would be noise in every locale file. `formatCategorizedLabel` prefers a
         * real translation the moment one exists under this key.
         *
         * Prints the front-end registry only. The Rust half is one `invoke('perf_snapshot')` away
         * in the same console (`docs/debugging.md` §4.1·§4.3) and cannot be read from here — `shared`
         * may not reach the `entities` layer that owns IPC.
         */
        titleDefaultValue: 'Show Performance Snapshot',
        categoryKey: KEYMAP_CATEGORY.APP,
        run: () => printPerfReport(),
        isEnabled: isPerfEnabled,
    },
    {
        id: 'tab.moveToNewWindow',
        titleKey: 'tab.moveToNewWindow',
        categoryKey: KEYMAP_CATEGORY.TAB,
        run: () => requestEditorPaneCommand({ type: 'move-focused-tab-to-window', target: { kind: 'newAuxiliary' } }),
    },
    {
        id: 'tab.moveToMainWindow',
        titleKey: 'tab.moveToMainWindow',
        categoryKey: KEYMAP_CATEGORY.TAB,
        run: () => requestEditorPaneCommand({ type: 'move-focused-tab-to-window', target: { kind: 'main' } }),
        /**
         * Only meaningful from inside an auxiliary window — this app's command palette is only
         * mounted in the main window (`app.tsx`, Wave I contract §3.1: `widgets/command-palette`
         * reads the global active-project session, which an auxiliary window deliberately never
         * queries), so in practice this stays disabled wherever it's actually shown today. Gated on
         * window context (rather than reusing `tab.close`'s hardcoded-off `alwaysDisabled`) so the
         * command is genuinely runnable the moment any surface mounts the palette inside an
         * auxiliary window; the tab context menu (`tab-context-menu.tsx`) already exercises the
         * same "Move back to Main Window" action unconditionally there.
         */
        isEnabled: () => getWindowContext().kind === 'auxiliary',
    },
    {
        id: 'view.toggleZenMode',
        titleKey: 'keymap.toggleZenMode',
        categoryKey: KEYMAP_CATEGORY.VIEW,
        keymapId: 'toggle-zen-mode',
        run: () => requestToggleZenMode(),
        /**
         * `ProjectLayout::shell_view` is main-window-only (contract §3.2) — same rationale as
         * `tab.moveToMainWindow` right above disabling itself outside an auxiliary window, mirrored
         * here in the other direction.
         */
        isEnabled: () => getWindowContext().kind !== 'auxiliary',
    },
]
