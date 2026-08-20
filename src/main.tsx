import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from '@app/app'
import { installRemoteInternalsShim } from '@shared/lib/remote/tauri-internals-shim'
import { ErrorBoundary } from '@shared/ui/error-boundary'
import '@shared/styles/global.css'

installRemoteInternalsShim()

const container = document.getElementById('root')

if (!container) throw new Error('root container not found')

createRoot(container).render(
    <StrictMode>
        <ErrorBoundary labelKey='errorBoundary.app'>
            <App />
        </ErrorBoundary>
    </StrictMode>,
)
