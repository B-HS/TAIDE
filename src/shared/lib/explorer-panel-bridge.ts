export type ExplorerViewId = 'files' | 'git'

type VoidListener = () => void
type ExplorerViewListener = (view: ExplorerViewId) => void

const toggleSidebarListeners = new Set<VoidListener>()

export const requestToggleExplorerSidebar = () => {
    for (const listener of toggleSidebarListeners) listener()
}

export const subscribeToggleExplorerSidebar = (listener: VoidListener) => {
    toggleSidebarListeners.add(listener)
    return () => {
        toggleSidebarListeners.delete(listener)
    }
}

const showViewListeners = new Set<ExplorerViewListener>()

export const requestShowExplorerView = (view: ExplorerViewId) => {
    for (const listener of showViewListeners) listener(view)
}

export const subscribeShowExplorerView = (listener: ExplorerViewListener) => {
    showViewListeners.add(listener)
    return () => {
        showViewListeners.delete(listener)
    }
}
