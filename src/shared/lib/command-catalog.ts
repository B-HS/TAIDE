import { toast } from 'sonner'
import type { AppCommand } from '@shared/lib/command-registry'
import { requestEditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'
import { requestShowExplorerView, requestToggleExplorerSidebar } from '@shared/lib/bridge/explorer-panel-bridge'
import { copyTextToClipboard } from '@shared/lib/copy-text-to-clipboard'
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
            if (await copyTextToClipboard(buildImeDebugReport())) toast.success(i18next.t('terminal.imeDebugCopied'))
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
         * Only meaningful from inside an auxiliary window, so it is gated on window context rather
         * than on `tab.close`'s hardcoded-off `alwaysDisabled`. That gate used to be theoretical —
         * the palette was main-window-only while it read the global active-project session — and is
         * live as of d-62 §1.D, which gave both dialogs a `projectId` prop and mounted them in the
         * auxiliary branch too (`app.tsx`). The tab context menu (`tab-context-menu.tsx`) offers the
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
    {
        id: 'tab.previousEditor',
        titleKey: 'keymap.editorPrevious',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'editor-previous',
        run: () => requestEditorPaneCommand({ type: 'cycle-tab', direction: 'prev' }),
    },
    {
        id: 'tab.nextEditor',
        titleKey: 'keymap.editorNext',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'editor-next',
        run: () => requestEditorPaneCommand({ type: 'cycle-tab', direction: 'next' }),
    },
    {
        id: 'editor.focusGroupLeft',
        titleKey: 'keymap.focusGroupLeft',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-left',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'direction', direction: 'left' } }),
    },
    {
        id: 'editor.focusGroupRight',
        titleKey: 'keymap.focusGroupRight',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-right',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'direction', direction: 'right' } }),
    },
    {
        id: 'editor.focusGroupUp',
        titleKey: 'keymap.focusGroupUp',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-up',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'direction', direction: 'up' } }),
    },
    {
        id: 'editor.focusGroupDown',
        titleKey: 'keymap.focusGroupDown',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-down',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'direction', direction: 'down' } }),
    },
    {
        id: 'tab.moveToGroupLeft',
        titleKey: 'keymap.moveTabToGroupLeft',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'move-tab-to-group-left',
        run: () => requestEditorPaneCommand({ type: 'move-tab-to-group', direction: 'left' }),
    },
    {
        id: 'tab.moveToGroupRight',
        titleKey: 'keymap.moveTabToGroupRight',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'move-tab-to-group-right',
        run: () => requestEditorPaneCommand({ type: 'move-tab-to-group', direction: 'right' }),
    },
    {
        id: 'tab.closeAllInGroup',
        titleKey: 'keymap.closeAllTabs',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'close-all-tabs',
        run: () => requestEditorPaneCommand({ type: 'close-all-tabs' }),
    },
    /**
     * ⌘1..⌘9 focus the n-th editor group in leaf order (`collectPaneLeaves`). Spelled out one entry
     * per position rather than generated: `keymapId` is a literal member of `KeymapActionId` and
     * `titleKey` a literal locale key, both of which a `map` over a range would widen to `string`,
     * costing the compile-time guarantee that every command points at a real keymap entry — the
     * same reason `MONACO_ACTIONS` lists its ~200 rows literally.
     */
    {
        id: 'editor.focusGroup1',
        titleKey: 'keymap.focusGroup1',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-1',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 1 } }),
    },
    {
        id: 'editor.focusGroup2',
        titleKey: 'keymap.focusGroup2',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-2',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 2 } }),
    },
    {
        id: 'editor.focusGroup3',
        titleKey: 'keymap.focusGroup3',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-3',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 3 } }),
    },
    {
        id: 'editor.focusGroup4',
        titleKey: 'keymap.focusGroup4',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-4',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 4 } }),
    },
    {
        id: 'editor.focusGroup5',
        titleKey: 'keymap.focusGroup5',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-5',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 5 } }),
    },
    {
        id: 'editor.focusGroup6',
        titleKey: 'keymap.focusGroup6',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-6',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 6 } }),
    },
    {
        id: 'editor.focusGroup7',
        titleKey: 'keymap.focusGroup7',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-7',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 7 } }),
    },
    {
        id: 'editor.focusGroup8',
        titleKey: 'keymap.focusGroup8',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-8',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 8 } }),
    },
    {
        id: 'editor.focusGroup9',
        titleKey: 'keymap.focusGroup9',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'focus-group-9',
        run: () => requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 9 } }),
    },
]
