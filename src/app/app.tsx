import '@app/bootstrap-commands'
import '@app/bootstrap-lsp'
import '@app/bootstrap-snippets'
import { AgentExternalOpenProvider } from '@app/providers/agent-external-open-provider'
import { AppProviders } from '@app/providers/app-providers'
import { EmmetProvider } from '@app/providers/emmet-provider'
import { HotExitFlushProvider } from '@app/providers/hot-exit-flush-provider'
import { IpcSyncProvider } from '@app/providers/ipc-sync-provider'
import { LocaleProvider } from '@app/providers/locale-provider'
import { ThemeProvider } from '@app/providers/theme-provider'
import { getWindowContext } from '@shared/lib/window-context'
import { AppShell } from '@widgets/app-shell/app-shell'
import { AuxiliaryWindowShell } from '@widgets/auxiliary-window-shell/auxiliary-window-shell'
import { CommandPalette } from '@widgets/command-palette/command-palette'
import { KeybindingsEditor } from '@widgets/keybindings-editor/keybindings-editor'
import { TaskRunnerDialog } from '@widgets/task-runner/task-runner-dialog'

/**
 * Branches the whole provider tree on `getWindowContext()` (contract §3.1) — an auxiliary editor
 * window renders `AuxiliaryWindowShell` pinned to its own `(projectId, windowSlot)` instead of
 * `AppShell`, and skips four things the main-window tree mounts:
 *
 * - `AgentExternalOpenProvider`: its `openProject`/`activateProject` calls mutate the single global
 *   active-project session, which an auxiliary window must never do to itself (it stays pinned to
 *   its own project regardless of what the main window has active).
 * - `CommandPalette` / `KeybindingsEditor` / `TaskRunnerDialog`: all three key off
 *   `activeProjectQueryOptions()` internally (that same global session), not this window's fixed
 *   project — rescoping them per-window is a `widgets/command-palette`/`widgets/keybindings-editor`/
 *   `widgets/task-runner` change outside this wave's scope (see the Wave I contract's F1 open
 *   issues). "Move Tab" is still reachable from an auxiliary window via the tab context menu
 *   (`tab-context-menu.tsx`), which needs none of those three.
 *
 * `IpcSyncProvider`, `HotExitFlushProvider`, `LocaleProvider`, `ThemeProvider`, and `EmmetProvider`
 * stay for both branches — none of them read the global active-project session, and
 * `layout:changed`/hot-exit flush/theme+locale sync/Emmet all need to reach every window.
 */
export const App = () => {
    const windowContext = getWindowContext()

    if (windowContext.kind === 'auxiliary') {
        return (
            <AppProviders>
                <IpcSyncProvider>
                    <HotExitFlushProvider>
                        <LocaleProvider>
                            <ThemeProvider>
                                <EmmetProvider>
                                    <AuxiliaryWindowShell projectId={windowContext.projectId} windowSlot={windowContext.windowSlot} />
                                </EmmetProvider>
                            </ThemeProvider>
                        </LocaleProvider>
                    </HotExitFlushProvider>
                </IpcSyncProvider>
            </AppProviders>
        )
    }

    return (
        <AppProviders>
            <IpcSyncProvider>
                <HotExitFlushProvider>
                    <AgentExternalOpenProvider>
                        <LocaleProvider>
                            <ThemeProvider>
                                <EmmetProvider>
                                    <AppShell />
                                    <CommandPalette />
                                    <KeybindingsEditor />
                                    <TaskRunnerDialog />
                                </EmmetProvider>
                            </ThemeProvider>
                        </LocaleProvider>
                    </AgentExternalOpenProvider>
                </HotExitFlushProvider>
            </IpcSyncProvider>
        </AppProviders>
    )
}
