import { createFireAndForgetBridge } from '@shared/lib/fire-and-forget-bridge'

export type ExplorerViewId = 'files' | 'git'

type VoidListener = () => void

const toggleExplorerSidebarBridge = createFireAndForgetBridge<undefined>()

export const requestToggleExplorerSidebar = () => toggleExplorerSidebarBridge.publish(undefined)
export const subscribeToggleExplorerSidebar = (listener: VoidListener) => toggleExplorerSidebarBridge.subscribe(() => listener())

const showExplorerViewBridge = createFireAndForgetBridge<ExplorerViewId>()

export const requestShowExplorerView = showExplorerViewBridge.publish
export const subscribeShowExplorerView = showExplorerViewBridge.subscribe
