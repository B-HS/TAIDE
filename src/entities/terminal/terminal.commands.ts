import type { AppCommand } from '@shared/lib/command-registry'
import { requestEditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap/keymap-category'

export const TERMINAL_COMMANDS: AppCommand[] = [
    {
        id: 'terminal.runSelectedText',
        titleKey: 'terminal.runSelectedText',
        categoryKey: KEYMAP_CATEGORY.TERMINAL,
        run: () => requestEditorPaneCommand({ type: 'run-selected-text-in-terminal' }),
    },
]
