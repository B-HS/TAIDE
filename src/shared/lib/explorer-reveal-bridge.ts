type RevealInExplorerListener = (path: string) => void

const listeners = new Set<RevealInExplorerListener>()

export const requestRevealInExplorer = (path: string) => {
    for (const listener of listeners) listener(path)
}

export const subscribeRevealInExplorer = (listener: RevealInExplorerListener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
