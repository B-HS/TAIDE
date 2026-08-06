import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { useDroppable } from '@dnd-kit/core'
import { Group, Panel } from 'react-resizable-panels'
import type { Layout, LayoutChangedMeta } from 'react-resizable-panels'
import type { DropEdge, PaneId, PaneNode, ProjectId } from '@shared/api/bindings'
import { useResizePane } from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import type { DropEdgeName } from '@features/split/split-drop-zones'
import { SplitDropZones } from '@features/split/split-drop-zones'
import { PaneSeparator } from '@features/split/pane-separator'
import { resolvePreviewKind } from '@shared/lib/preview-kind'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { PaneTabBar } from '@widgets/editor-area/pane-tab-bar'
import { DiffPane } from '@widgets/diff-pane/diff-pane'
import { EditorPane } from '@widgets/editor-pane/editor-pane'
import { PreviewPane } from '@widgets/preview-pane/preview-pane'
import { SettingsView } from '@widgets/settings-view/settings-view'
import { TerminalSession } from '@widgets/terminal-pane/terminal-session'

const EQUAL_SPLIT_TOTAL_PERCENT = 100
const MIN_PANEL_SIZE_PX = 120
const PATH_SEPARATOR = '/'

export type SplitDropData = { type: 'split'; paneId: PaneId; edge: DropEdge }

type PaneNodeViewProps = {
    node: PaneNode
    projectId: ProjectId
    focusedPaneId: PaneId
    isDragging: boolean
    overTarget: { paneId: PaneId; edge: DropEdge } | null
}

export const PaneNodeView: FC<PaneNodeViewProps> = ({ node, projectId, focusedPaneId, isDragging, overTarget }) => {
    const { t } = useTranslation()
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: resizePane } = useResizePane(projectId)
    const dropLeft = useDroppable({ id: `${node.id}:left`, data: { type: 'split', paneId: node.id, edge: 'left' } satisfies SplitDropData })
    const dropRight = useDroppable({ id: `${node.id}:right`, data: { type: 'split', paneId: node.id, edge: 'right' } satisfies SplitDropData })
    const dropTop = useDroppable({ id: `${node.id}:top`, data: { type: 'split', paneId: node.id, edge: 'top' } satisfies SplitDropData })
    const dropBottom = useDroppable({ id: `${node.id}:bottom`, data: { type: 'split', paneId: node.id, edge: 'bottom' } satisfies SplitDropData })
    const dropCenter = useDroppable({ id: `${node.id}:center`, data: { type: 'split', paneId: node.id, edge: 'center' } satisfies SplitDropData })

    if (node.node === 'split') {
        const children = node.children
        const handleLayoutChanged = (layout: Layout, meta: LayoutChangedMeta) => {
            if (!meta.isUserInteraction) return
            resizePane({ paneId: node.id, sizes: children.map((child) => layout[child.id] ?? EQUAL_SPLIT_TOTAL_PERCENT / children.length) })
        }

        const items = children.flatMap((child, index) => [
            index > 0 && (
                <PaneSeparator
                    key={`separator-${child.id}`}
                    orientation={node.dir === 'horizontal' ? 'horizontal' : 'vertical'}
                    thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS}
                />
            ),
            <Panel
                key={child.id}
                id={child.id}
                defaultSize={`${node.sizes[index] ?? EQUAL_SPLIT_TOTAL_PERCENT / children.length}%`}
                minSize={MIN_PANEL_SIZE_PX}
                className='min-h-0 min-w-0'>
                <PaneNodeView node={child} projectId={projectId} focusedPaneId={focusedPaneId} isDragging={isDragging} overTarget={overTarget} />
            </Panel>,
        ])

        return (
            <Group orientation={node.dir} onLayoutChanged={handleLayoutChanged} className='min-h-0 min-w-0 flex-1'>
                {items}
            </Group>
        )
    }

    const activeTab = node.tabs.find((tab) => tab.id === node.active) ?? null
    const activeFilePath = activeTab?.kind.kind === 'file' ? activeTab.kind.path : null
    const activeFileName = activeFilePath ? activeFilePath.slice(activeFilePath.lastIndexOf(PATH_SEPARATOR) + 1) : null
    const activePreviewKind = activeFileName ? resolvePreviewKind(activeFileName) : null
    const dropRefByEdge: Record<DropEdgeName, (element: HTMLElement | null) => void> = {
        left: dropLeft.setNodeRef,
        right: dropRight.setNodeRef,
        top: dropTop.setNodeRef,
        bottom: dropBottom.setNodeRef,
        center: dropCenter.setNodeRef,
    }

    return (
        <div className='flex h-full min-h-0 w-full min-w-0 flex-1 flex-col'>
            <PaneTabBar projectId={projectId} paneId={node.id} tabs={node.tabs} activeTabId={node.active} focused={node.id === focusedPaneId} />
            <div className='bg-editor-background text-editor-foreground relative min-h-0 flex-1 overflow-hidden'>
                {activeTab?.kind.kind === 'file' && activePreviewKind === null && (
                    <EditorPane projectId={projectId} tabId={activeTab.id} path={activeTab.kind.path} />
                )}
                {activeTab?.kind.kind === 'file' && activePreviewKind !== null && <PreviewPane key={activeTab.id} path={activeTab.kind.path} />}
                {activeTab?.kind.kind === 'terminal' && (
                    <TerminalSession key={activeTab.id} projectId={projectId} tabId={activeTab.id} sessionId={activeTab.kind.sessionId} />
                )}
                {activeTab?.kind.kind === 'settings' && <SettingsView />}
                {activeTab?.kind.kind === 'diff' && <DiffPane projectId={projectId} path={activeTab.kind.path} staged={activeTab.kind.staged} />}
                {activeTab &&
                    activeTab.kind.kind !== 'file' &&
                    activeTab.kind.kind !== 'terminal' &&
                    activeTab.kind.kind !== 'settings' &&
                    activeTab.kind.kind !== 'diff' && (
                        <div className='flex h-full w-full items-center justify-center text-sm opacity-60'>{activeTab.title}</div>
                    )}
                {!activeTab && <div className='flex h-full w-full items-center justify-center text-sm opacity-40'>{t('editor.noFileOpen')}</div>}
                {isDragging && (
                    <SplitDropZones
                        activeEdge={overTarget?.paneId === node.id ? overTarget.edge : null}
                        renderZone={(edge, className) => <div key={edge} ref={dropRefByEdge[edge]} className={className} />}
                    />
                )}
            </div>
        </div>
    )
}
