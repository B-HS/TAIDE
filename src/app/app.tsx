import { AppProviders } from '@app/providers/app-providers'
import { IpcSyncProvider } from '@app/providers/ipc-sync-provider'
import { ThemeProvider } from '@app/providers/theme-provider'
import { AppShell } from '@widgets/app-shell/app-shell'
import { CommandPalette } from '@widgets/command-palette/command-palette'

export const App = () => (
    <AppProviders>
        <IpcSyncProvider>
            <ThemeProvider>
                <AppShell />
                <CommandPalette />
            </ThemeProvider>
        </IpcSyncProvider>
    </AppProviders>
)
