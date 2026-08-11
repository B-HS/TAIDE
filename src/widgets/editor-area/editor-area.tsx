import type { FC } from 'react'
import { useEffect, useEffectEvent, useState } from 'react'
import { DndContext, DragOverlay, PointerSensor, pointerWithin, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/core'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import type { DropEdge, PaneId, ProjectId, TabId, TabKind } from '@shared/api/bindings'
import { getEditorInstance } from '@entities/editor/editor-instance-registry'
import { layoutQueryOptions, useActivateTab, useCloseTab, useMoveTab, useOpenTab, useSplitPane } from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import type { EditorPaneCommand, TabCycleDirection } from '@shared/lib/editor-pane-command-bridge'
import { subscribeEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { APP_KEYMAP, applyKeymapOverrides, parseKeymapOverrides } from '@shared/lib/keymap'
import { monaco } from '@shared/lib/monaco/setup'
import { findPaneLeaf, findPaneTab } from '@shared/lib/pane-tree'
import { requestOpenSearchPanel } from '@shared/lib/search-panel-bridge'
import { TabItem } from '@features/tab/tab-item'
import type { TabContainerDropData } from '@widgets/editor-area/pane-tab-bar'
import { getTabIcon } from '@widgets/editor-area/pane-tab-bar'
import type { SplitDropData } from '@widgets/editor-area/pane-node-view'
import { PaneNodeView } from '@widgets/editor-area/pane-node-view'
import type { TabDragData } from '@widgets/editor-area/sortable-tab'
import { ProblemsPanelContainer } from '@widgets/problems-panel/problems-panel-container'

const DRAG_ACTIVATION_DISTANCE_PX = 4

type OverDropData = SplitDropData | TabContainerDropData

type DragTabState = {
    id: TabId
    title: string
    kind: TabKind
    pinned: boolean
    preview: boolean
    dirty: boolean
}

type EditorAreaProps = {
    projectId: ProjectId
    isProblemsOpen: boolean
    onCloseProblems: () => void
}

export const EditorArea: FC<EditorAreaProps> = ({ projectId, isProblemsOpen, onCloseProblems }) => {
    const [dragTab, setDragTab] = useState<DragTabState | null>(null)
    const [overTarget, setOverTarget] = useState<{ paneId: PaneId; edge: DropEdge } | null>(null)

    const { t } = useTranslation()
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: moveTab } = useMoveTab(projectId)
    const { mutate: splitPane } = useSplitPane(projectId)
    const { mutate: closeTab } = useCloseTab(projectId)
    const { mutate: activateTab } = useActivateTab(projectId)
    const { mutate: openTab } = useOpenTab(projectId)

    const closeFocusedTab = () => {
        if (!layout) return
        const leaf = findPaneLeaf(layout.root, layout.focusedPane)
        if (!leaf?.active) return
        closeTab(leaf.active)
    }

    const openFind = () => {
        const focusedEditor = monaco.editor.getEditors().find((instance) => instance.hasTextFocus())
        if (focusedEditor) {
            focusedEditor.getAction('actions.find')?.run()
            return
        }
        requestOpenSearchPanel()
    }

    const openGlobalSearch = () => {
        const focusedEditor = monaco.editor.getEditors().find((instance) => instance.hasTextFocus())
        const selection = focusedEditor?.getSelection()
        const selectedText = selection && !selection.isEmpty() ? (focusedEditor?.getModel()?.getValueInRange(selection) ?? null) : null
        requestOpenSearchPanel({ seedText: selectedText && !selectedText.includes('\n') ? selectedText : null })
    }

    const splitActiveEditor = () => {
        if (!layout) return
        const leaf = findPaneLeaf(layout.root, layout.focusedPane)
        if (!leaf?.active) return
        splitPane({ paneId: layout.focusedPane, edge: 'right', tabId: leaf.active })
    }

    const cycleTab = (direction: TabCycleDirection) => {
        if (!layout) return
        const leaf = findPaneLeaf(layout.root, layout.focusedPane)
        if (!leaf?.active) return
        if (leaf.tabs.length < 2) return
        const currentIndex = leaf.tabs.findIndex((tab) => tab.id === leaf.active)
        if (currentIndex < 0) return
        const step = direction === 'next' ? 1 : -1
        const nextIndex = (currentIndex + step + leaf.tabs.length) % leaf.tabs.length
        activateTab(leaf.tabs[nextIndex].id)
    }

    const saveActiveTab = () => {
        if (!layout) return
        const leaf = findPaneLeaf(layout.root, layout.focusedPane)
        const activeTab = leaf?.tabs.find((tab) => tab.id === leaf.active)
        if (activeTab?.kind.kind !== 'file') return
        const targetEditor = getEditorInstance(activeTab.id)
        targetEditor?.getAction('taide.saveFile')?.run()
    }

    const toggleTerminal = () => {
        if (!layout) return
        const leaf = findPaneLeaf(layout.root, layout.focusedPane)
        if (!leaf) return
        const activeTab = leaf.tabs.find((tab) => tab.id === leaf.active)
        if (activeTab?.kind.kind === 'terminal') {
            const fallbackTab = leaf.tabs.find((tab) => tab.id !== leaf.active)
            if (fallbackTab) activateTab(fallbackTab.id)
            return
        }
        const terminalTab = leaf.tabs.find((tab) => tab.kind.kind === 'terminal')
        if (terminalTab) {
            activateTab(terminalTab.id)
            return
        }
        openTab(
            { projectId, kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
        )
    }

    const keymapEntries = applyKeymapOverrides(APP_KEYMAP, parseKeymapOverrides(settings?.keymapOverrides ?? null))

    useGlobalKeymap(
        {
            'close-tab': closeFocusedTab,
            find: openFind,
            search: openGlobalSearch,
            'search-replace': () => requestOpenSearchPanel({ openReplace: true }),
            split: splitActiveEditor,
            'tab-cycle-next': () => cycleTab('next'),
            'tab-cycle-prev': () => cycleTab('prev'),
            save: saveActiveTab,
            'toggle-terminal': toggleTerminal,
        },
        keymapEntries,
    )

    const handleEditorPaneCommand = useEffectEvent((command: EditorPaneCommand) => {
        if (command.type === 'split') return splitActiveEditor()
        if (command.type === 'cycle-tab') return cycleTab(command.direction)
        if (command.type === 'save-active-tab') return saveActiveTab()
        if (command.type === 'toggle-terminal') return toggleTerminal()
    })

    useEffect(() => subscribeEditorPaneCommand(handleEditorPaneCommand), [])

    const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_DISTANCE_PX } }))

    const handleDragStart = ({ active }: DragStartEvent) => {
        if (!layout) return
        const tab = findPaneTab(layout.root, active.id as TabId)
        if (!tab) return
        setDragTab({
            id: tab.id,
            title: tab.title,
            kind: tab.kind,
            pinned: tab.pinned ?? false,
            preview: tab.preview ?? false,
            dirty: tab.dirty ?? false,
        })
    }

    const handleDragOver = ({ over }: DragOverEvent) => {
        const data = over?.data.current as OverDropData | TabDragData | undefined
        setOverTarget(data?.type === 'split' ? { paneId: data.paneId, edge: data.edge } : null)
    }

    const handleDragCancel = () => {
        setDragTab(null)
        setOverTarget(null)
    }

    const handleDragEnd = ({ active, over }: DragEndEvent) => {
        setDragTab(null)
        setOverTarget(null)
        if (!over || !layout) return

        const tabId = active.id as TabId
        const activeData = active.data.current as TabDragData | undefined
        const overData = over.data.current as OverDropData | TabDragData | undefined
        if (!activeData || !overData) return

        if (overData.type === 'split') {
            if (overData.edge === 'center') {
                const leaf = findPaneLeaf(layout.root, overData.paneId)
                if (leaf) moveTab({ tabId, paneId: overData.paneId, index: leaf.tabs.length })
                return
            }
            splitPane({ paneId: overData.paneId, edge: overData.edge, tabId })
            return
        }

        if (overData.type === 'tab-container') {
            const leaf = findPaneLeaf(layout.root, overData.paneId)
            if (leaf) moveTab({ tabId, paneId: overData.paneId, index: leaf.tabs.length })
            return
        }

        const leaf = findPaneLeaf(layout.root, overData.paneId)
        if (!leaf) return
        const rawIndex = leaf.tabs.findIndex((tab) => tab.id === over.id)
        if (rawIndex < 0) return
        const pinnedCount = leaf.tabs.filter((tab) => tab.pinned).length
        const index = activeData.pinned ? Math.min(rawIndex, pinnedCount) : Math.max(rawIndex, pinnedCount)
        moveTab({ tabId, paneId: overData.paneId, index })
    }

    if (!layout) return <div className='bg-editor-background h-full w-full' />

    return (
        <DndContext
            sensors={sensors}
            collisionDetection={pointerWithin}
            onDragStart={handleDragStart}
            onDragOver={handleDragOver}
            onDragEnd={handleDragEnd}
            onDragCancel={handleDragCancel}>
            <Group orientation='vertical' className='min-h-0 min-w-0 flex-1'>
                <Panel id='editor-panes' minSize='30%' className='min-h-0 min-w-0'>
                    <div className='relative flex h-full min-h-0 w-full min-w-0 overflow-hidden'>
                        <PaneNodeView
                            node={layout.root}
                            projectId={projectId}
                            focusedPaneId={layout.focusedPane}
                            isDragging={!!dragTab}
                            overTarget={overTarget}
                        />
                    </div>
                </Panel>
                {isProblemsOpen && <PaneSeparator orientation='vertical' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />}
                {isProblemsOpen && (
                    <Panel id='problems-panel' defaultSize='220px' minSize='120px' className='min-h-0 min-w-0'>
                        <ProblemsPanelContainer projectId={projectId} onClose={onCloseProblems} />
                    </Panel>
                )}
            </Group>
            <DragOverlay>
                {dragTab && (
                    <div className='pointer-events-none opacity-90'>
                        <TabItem
                            title={dragTab.title}
                            icon={getTabIcon(dragTab.kind)}
                            active
                            dirty={dragTab.dirty}
                            pinned={dragTab.pinned}
                            preview={dragTab.preview}
                            onActivate={() => {}}
                            onClose={() => {}}
                        />
                    </div>
                )}
            </DragOverlay>
        </DndContext>
    )
}
