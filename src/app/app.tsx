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
import { AppShell } from '@widgets/app-shell/app-shell'
import { CommandPalette } from '@widgets/command-palette/command-palette'
import { KeybindingsEditor } from '@widgets/keybindings-editor/keybindings-editor'
import { TaskRunnerDialog } from '@widgets/task-runner/task-runner-dialog'

export const App = () => (
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
