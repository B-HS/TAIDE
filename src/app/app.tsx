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
 * - `CommandPalette` / `TaskRunnerDialog`: both key off `activeProjectQueryOptions()` internally
 *   (that same global session), not this window's fixed project — rescoping them per-window is a
 *   `widgets/command-palette`/`widgets/task-runner` change outside this wave's scope (see the Wave I
 *   contract's F1 open issues). "Move Tab" is still reachable from an auxiliary window via the tab
 *   context menu (`tab-context-menu.tsx`), which needs neither of those two.
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
                        <AgentExternalOpenProvider>
                            <IdeSyncProvider>
                                <AgentStateSyncProvider>
                                    <LocaleProvider>
                                        <ThemeProvider>
                                            <EmmetProvider>
                                                <KeybindingsRuntimeProvider>
                                                    <AppShell />
                                                    <CommandPalette />
                                                    <TaskRunnerDialog />
                                                </KeybindingsRuntimeProvider>
                                            </EmmetProvider>
                                        </ThemeProvider>
                                    </LocaleProvider>
                                </AgentStateSyncProvider>
                            </IdeSyncProvider>
                        </AgentExternalOpenProvider>
                    </HotExitFlushProvider>
                </IpcSyncProvider>
            </ExternalLinkProvider>
        </AppProviders>
    )
}
