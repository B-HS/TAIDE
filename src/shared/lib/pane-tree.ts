import type { PaneId, PaneNode, Tab, TabId } from '@shared/api/bindings'

export const findPaneLeaf = (node: PaneNode, paneId: PaneId): Extract<PaneNode, { node: 'leaf' }> | null => {
    if (node.node === 'leaf') return node.id === paneId ? node : null
    for (const child of node.children) {
        const found = findPaneLeaf(child, paneId)
        if (found) return found
    }
    return null
}

export const findPaneTab = (node: PaneNode, tabId: TabId): Tab | null => {
    if (node.node === 'leaf') return node.tabs.find((tab) => tab.id === tabId) ?? null
    for (const child of node.children) {
        const found = findPaneTab(child, tabId)
        if (found) return found
    }
    return null
}

export const findActiveTab = (node: PaneNode, paneId: PaneId): Tab | null => {
    const leaf = findPaneLeaf(node, paneId)
    if (!leaf?.active) return null
    return leaf.tabs.find((tab) => tab.id === leaf.active) ?? null
}
