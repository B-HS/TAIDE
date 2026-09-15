import '@app/bootstrap-commands'
import '@app/bootstrap-lsp'
import '@app/bootstrap-snippets'
import { AgentExternalOpenProvider } from '@app/providers/agent-external-open-provider'
import { AgentStateSyncProvider } from '@app/providers/agent-state-sync-provider'
import { AppProviders } from '@app/providers/app-providers'
import { EmmetProvider } from '@app/providers/emmet-provider'
import { ExternalLinkProvider } from '@app/providers/external-link-provider'
import { HotExitFlushProvider } from '@app/providers/hot-exit-flush-provider'
import { IdeSyncProvider } from '@app/providers/ide-sync-provider'
import { IpcSyncProvider } from '@app/providers/ipc-sync-provider'
import { KeybindingsRuntimeProvider } from '@app/providers/keybindings-runtime-provider'
import { LocaleProvider } from '@app/providers/locale-provider'
import { MainWindowDialogs } from '@app/main-window-dialogs'
import { NativeNotificationProvider } from '@app/providers/native-notification-provider'
import { ShellSlotProvider } from '@app/providers/shell-slot-provider'
import { ThemeProvider } from '@app/providers/theme-provider'
import { getWindowContext } from '@shared/lib/window-context'
import { AppShell } from '@widgets/app-shell/app-shell'
import { AuxiliaryWindowShell } from '@widgets/auxiliary-window-shell/auxiliary-window-shell'
import { CommandPalette } from '@widgets/command-palette/command-palette'
import { TaskRunnerDialog } from '@widgets/task-runner/task-runner-dialog'

/**
 * Branches the whole provider tree on `getWindowContext()` (contract §3.1) — an auxiliary editor
 * window renders `AuxiliaryWindowShell` pinned to its own `(projectId, windowSlot)` instead of
 * `AppShell`, and skips three things the main-window tree mounts:
 *
 * - `AgentExternalOpenProvider`: its `openProject`/`activateProject` calls mutate the single global
 *   active-project session, which an auxiliary window must never do to itself (it stays pinned to
 *   its own project regardless of what the main window has active).
 * - `IdeSyncProvider`: the backend broadcasts the Claude Code IDE protocol events it handles
 *   (`ide:diff-requested`/`ide:save-requested`/`ide:close-tab-requested`) to every window via a
 *   plain `.emit(app)`, so mounting it in both branches would race both windows to open/resolve the
 *   same request twice; its diagnostics push also reads the global active-project session, same as
 *   `AgentExternalOpenProvider` above.
 * - `NativeNotificationProvider`: the backend broadcasts `agent:state-changed` and
 *   `lsp:install-progress` to every window, so mounting it in both branches would send the same OS
 *   notification once per open window — one realm has to own the channel, and the main window is
 *   the one guaranteed to exist for the whole session (`native-notification-gate.ts`).
 *
 * `CommandPalette`/`TaskRunnerDialog` are mounted in *both* branches as of d-62 §1.D: each now
 * takes the project it acts on as a prop — the main window's active project (`MainWindowDialogs`),
 * an auxiliary window's own fixed one — instead of reading the global active-project session
 * itself, which was the sole reason they used to be main-window-only (Wave I F1's open issue).
 *
 * `ShellSlotProvider` is likewise main-window-only: it owns which of the main window's shell slots
 * has focus (d-62 §1.B), and an auxiliary window has no slot tree at all. Everything that reads it
 * treats "no provider" as "one shell, always focused", which is exactly an auxiliary window.
 *
 * `ExternalLinkProvider`, `IpcSyncProvider`, `HotExitFlushProvider`, `AgentStateSyncProvider`,
 * `LocaleProvider`, `ThemeProvider`, `EmmetProvider`, and `KeybindingsRuntimeProvider` stay for both
 * branches — none of them read the global active-project session: `layout:changed`/hot-exit
 * flush/theme+locale sync/Emmet all need to reach every window, an off-origin anchor strands
 * whichever window rendered it, agent status pushes are keyed by `payload.projectId`
 * (read by `PaneTabBar`'s agent badge in both `AppShell` and `AuxiliaryWindowShell`, via
 * `EditorArea`), and `KeybindingsRuntimeProvider` applies monaco keybinding overrides to *this*
 * window's own monaco instance and answers *this* window's own local "open keymap editor" shortcut
 * (see that provider's own doc comment — unlike `IdeSyncProvider` there is no cross-window
 * broadcast to race, since a bridge publish is per-realm module state, not backend-emitted).
 */
export const App = () => {
    const windowContext = getWindowContext()

    if (windowContext.kind === 'auxiliary') {
        return (
            <AppProviders>
                <ExternalLinkProvider>
                    <IpcSyncProvider>
                        <HotExitFlushProvider>
                            <AgentStateSyncProvider>
                                <LocaleProvider>
                                    <ThemeProvider>
                                        <EmmetProvider>
                                            <KeybindingsRuntimeProvider>
                                                <AuxiliaryWindowShell projectId={windowContext.projectId} windowSlot={windowContext.windowSlot} />
                                                <CommandPalette projectId={windowContext.projectId} />
                                                <TaskRunnerDialog projectId={windowContext.projectId} />
                                            </KeybindingsRuntimeProvider>
                                        </EmmetProvider>
                                    </ThemeProvider>
                                </LocaleProvider>
                            </AgentStateSyncProvider>
                        </HotExitFlushProvider>
                    </IpcSyncProvider>
                </ExternalLinkProvider>
            </AppProviders>
        )
    }

    return (
        <AppProviders>
            <ExternalLinkProvider>
                <IpcSyncProvider>
                    <HotExitFlushProvider>
                        <ShellSlotProvider>
                            <AgentExternalOpenProvider>
                                <IdeSyncProvider>
                                    <AgentStateSyncProvider>
                                        <LocaleProvider>
                                            <ThemeProvider>
                                                <EmmetProvider>
                                                    <KeybindingsRuntimeProvider>
                                                        <NativeNotificationProvider>
                                                            <AppShell />
                                                            <MainWindowDialogs />
                                                        </NativeNotificationProvider>
                                                    </KeybindingsRuntimeProvider>
                                                </EmmetProvider>
                                            </ThemeProvider>
                                        </LocaleProvider>
                                    </AgentStateSyncProvider>
                                </IdeSyncProvider>
                            </AgentExternalOpenProvider>
                        </ShellSlotProvider>
                    </HotExitFlushProvider>
                </IpcSyncProvider>
            </ExternalLinkProvider>
        </AppProviders>
    )
}
