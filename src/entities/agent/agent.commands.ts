import { toast } from 'sonner'
import type { AppCommand } from '@shared/lib/command-registry'
import { IS_MAC } from '@shared/constants/platform'
import { i18next } from '@shared/i18n/i18n'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap-category'
import { installCliCommand, uninstallCliCommand } from '@entities/agent/agent.ipc'

const runCliInstall = async () => {
    try {
        await installCliCommand()
        toast.success(i18next.t('settings.cliInstallSuccess'))
    } catch {
        toast.error(i18next.t('settings.cliInstallFailed'))
    }
}

const runCliUninstall = async () => {
    try {
        await uninstallCliCommand()
        toast.success(i18next.t('settings.cliUninstallSuccess'))
    } catch {
        toast.error(i18next.t('settings.cliUninstallFailed'))
    }
}

/**
 * macOS-only, mirroring VS Code's `workbench.action.install/uninstallCommandLine` — the CLI
 * shell-command install/uninstall itself is rejected by the backend on other platforms, so these
 * commands are not registered there at all (bootstrap-commands.ts §3.3 guard).
 */
export const AGENT_CLI_COMMANDS: AppCommand[] = IS_MAC
    ? [
          { id: 'cli.installShellCommand', titleKey: 'keymap.cliInstall', categoryKey: KEYMAP_CATEGORY.SHELL_COMMAND, run: runCliInstall },
          { id: 'cli.uninstallShellCommand', titleKey: 'keymap.cliUninstall', categoryKey: KEYMAP_CATEGORY.SHELL_COMMAND, run: runCliUninstall },
      ]
    : []
