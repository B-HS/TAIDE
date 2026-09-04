import type { Page } from '@playwright/test'
import { invokeIpc } from './ipc'

/**
 * The slice of `ProjectLayout` (`src/shared/api/bindings.ts`) the specs assert on — a pane tree
 * whose leaves hold tabs. Kept as a structural subset rather than importing the generated bindings,
 * for the same decoupling reason `invokeIpc` documents.
 */
export type PaneTab = {
    id: string
    title: string
    kind: { kind: string }
}

export type PaneLeaf = { node: 'leaf'; id: string; tabs: PaneTab[]; active: string | null }

export type PaneNode = PaneLeaf | { node: 'split'; id: string; dir: 'horizontal' | 'vertical'; children: PaneNode[] }

export type ProjectLayout = { root: PaneNode }

export const readProjectLayout = (page: Page, projectId: string) => invokeIpc<ProjectLayout>(page, 'layout_get', { projectId })

export const collectPaneLeaves = (node: PaneNode): PaneLeaf[] => (node.node === 'leaf' ? [node] : node.children.flatMap(collectPaneLeaves))

export const collectPaneTabs = (layout: ProjectLayout) => collectPaneLeaves(layout.root).flatMap((leaf) => leaf.tabs)

export const countTabsOfKind = (layout: ProjectLayout, kind: string) => collectPaneTabs(layout).filter((tab) => tab.kind.kind === kind).length
