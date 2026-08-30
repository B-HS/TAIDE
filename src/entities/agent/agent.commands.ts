import { toast } from 'sonner'
import type { AppCommand } from '@shared/lib/command-registry'
import { IS_MAC } from '@shared/constants/platform'
import { i18next } from '@shared/i18n/i18n'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap/keymap-category'
import { getCliInstallStatus, installCliCommand, uninstallCliCommand } from '@entities/agent/agent.ipc'

/**
 * Claude Code's ctrl+g opens `$EDITOR <tmpfile>` and waits for it to exit. Every terminal TAIDE
 * spawns already carries an `EDITOR` pointing at the `taide` CLI (Rust side:
 * `agent::commands::editor_terminal_env`), so the only thing left to arrange is the CLI itself —
 * install it when it is missing (or its symlink went stale), then confirm that terminals opened
 * from now on round-trip through this window. The install resolves to the status it left behind
 * (dismissing the administrator prompt is a quiet no-op, not an error), so the connected toast is
 * only claimed once that status actually reports a live symlink. External terminals need `EDITOR`
 * exported by hand instead: `docs/features/agent-integration.md` §2.3.
 */
const runConnectExternalEditor = async () => {
    try {
        const status = await getCliInstallStatus()
        const connected = status.installed && !status.dangling ? status : await installCliCommand()
        if (!connected.installed || connected.dangling) {
            toast.error(i18next.t('settings.cliInstallFailed'))
            return
        }
        toast.success(i18next.t('settings.cliExternalEditorConnected'))
    } catch {
        toast.error(i18next.t('settings.cliInstallFailed'))
    }
}

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
          {
              id: 'cli.connectExternalEditor',
              titleKey: 'keymap.cliConnectExternalEditor',
              categoryKey: KEYMAP_CATEGORY.SHELL_COMMAND,
              run: runConnectExternalEditor,
          },
          { id: 'cli.installShellCommand', titleKey: 'keymap.cliInstall', categoryKey: KEYMAP_CATEGORY.SHELL_COMMAND, run: runCliInstall },
          { id: 'cli.uninstallShellCommand', titleKey: 'keymap.cliUninstall', categoryKey: KEYMAP_CATEGORY.SHELL_COMMAND, run: runCliUninstall },
      ]
    : []
