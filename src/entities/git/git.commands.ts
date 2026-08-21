import { toast } from 'sonner'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { requestOpenCreateTagDialog } from '@shared/lib/create-tag-dialog-bridge'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { requestShowExplorerView } from '@shared/lib/explorer-panel-bridge'
import { i18next } from '@shared/i18n/i18n'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap-category'
import { OPEN_FILE_HISTORY_MONACO_ACTION_ID, TOGGLE_BLAME_MONACO_ACTION_ID } from '@entities/git/git.constant'
import { revertGitCommit } from '@entities/git/git.ipc'

const HEAD_REV = 'HEAD'

const runRevertHead = async (context: CommandContext) => {
    if (!context.activeProjectId) return
    try {
        const outcome = await revertGitCommit({ projectId: context.activeProjectId, rev: HEAD_REV })
        toast[outcome.conflicted ? 'warning' : 'success'](i18next.t(outcome.conflicted ? 'git.revertConflict' : 'git.revertSuccess'))
    } catch (error) {
        toast.error(error instanceof Error ? error.message : i18next.t('git.revert'))
    }
}

const runCreateTagOnHead = () => {
    requestShowExplorerView('git')
    requestOpenCreateTagDialog({ target: HEAD_REV })
}

const isProjectActive = (context: CommandContext) => context.activeProjectId !== null

export const GIT_COMMANDS: AppCommand[] = [
    { id: 'git.revertHead', titleKey: 'git.revert', categoryKey: KEYMAP_CATEGORY.GIT, run: runRevertHead, isEnabled: isProjectActive },
    { id: 'git.createTagOnHead', titleKey: 'git.createTag', categoryKey: KEYMAP_CATEGORY.GIT, run: runCreateTagOnHead, isEnabled: isProjectActive },
    {
        id: 'git.toggleBlame',
        titleKey: 'git.toggleBlame',
        categoryKey: KEYMAP_CATEGORY.GIT,
        run: () => requestEditorPaneCommand({ type: 'run-monaco-action', actionId: TOGGLE_BLAME_MONACO_ACTION_ID }),
    },
    {
        id: 'git.openFileHistory',
        titleKey: 'git.fileHistory',
        categoryKey: KEYMAP_CATEGORY.GIT,
        run: () => requestEditorPaneCommand({ type: 'run-monaco-action', actionId: OPEN_FILE_HISTORY_MONACO_ACTION_ID }),
    },
]
