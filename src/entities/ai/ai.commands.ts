import type { AppCommand } from '@shared/lib/command-registry'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap-category'
import { AI_INLINE_EDIT_MONACO_ACTION_ID } from '@entities/ai/ai.constant'

export const AI_COMMANDS: AppCommand[] = [
    {
        id: 'ai.inlineEdit',
        titleKey: 'ai.inlineEditPaletteLabel',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        run: () => requestEditorPaneCommand({ type: 'run-monaco-action', actionId: AI_INLINE_EDIT_MONACO_ACTION_ID }),
        isEnabled: (context) => context.activeEditorActionIds !== null,
    },
]
