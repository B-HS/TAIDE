import '@shared/lib/error-log-forwarding-install'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { App } from '@app/app'
import { installRemoteInternalsShim } from '@shared/lib/remote/tauri-internals-shim'
import { ErrorBoundary } from '@shared/ui/error-boundary'
import '@shared/styles/global.css'

installRemoteInternalsShim()

const container = document.getElementById('root')

if (!container) throw new Error('root container not found')

const revealWindowOnRootCrash = () => {
    document.documentElement.dataset.themeReady = ''
    document.documentElement.dataset.localeReady = ''
    void getCurrentWindow().show()
}

createRoot(container).render(
    <StrictMode>
        <ErrorBoundary labelKey='errorBoundary.app' labelFallback='Application' onCaught={revealWindowOnRootCrash}>
            <App />
        </ErrorBoundary>
    </StrictMode>,
)
