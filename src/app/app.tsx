import '@app/bootstrap-commands'
import { AgentExternalOpenProvider } from '@app/providers/agent-external-open-provider'
import { AppProviders } from '@app/providers/app-providers'
import { IpcSyncProvider } from '@app/providers/ipc-sync-provider'
import { LocaleProvider } from '@app/providers/locale-provider'
import { ThemeProvider } from '@app/providers/theme-provider'
import { AppShell } from '@widgets/app-shell/app-shell'
import { CommandPalette } from '@widgets/command-palette/command-palette'
import { KeybindingsEditor } from '@widgets/keybindings-editor/keybindings-editor'

export const App = () => (
    <AppProviders>
        <IpcSyncProvider>
            <AgentExternalOpenProvider>
                <LocaleProvider>
                    <ThemeProvider>
                        <AppShell />
                        <CommandPalette />
                        <KeybindingsEditor />
                    </ThemeProvider>
                </LocaleProvider>
            </AgentExternalOpenProvider>
        </IpcSyncProvider>
    </AppProviders>
)
