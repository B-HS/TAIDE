import type { FC } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { DndContext, DragOverlay, PointerSensor, pointerWithin, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/core'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import type { DropEdge, PaneId, ProjectId, TabId, TabKind } from '@shared/api/bindings'
import { getEditorInstance, subscribeEditorInstance } from '@entities/editor/editor-instance-registry'
import { pruneMirrors, pruneUntitledMirrors } from '@entities/file/file.ipc'
import { layoutQueryOptions, useActivateTab, useCloseTab, useMoveTab, useOpenTab, useSplitPane } from '@entities/layout/layout.query'
import { requestReveal } from '@entities/editor/reveal-registry'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { setActiveEditorActionIds } from '@shared/lib/active-editor-actions-bridge'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { QUERY_KEY } from '@shared/constants/query-key'
import { resolveSelectedTextOrCurrentLine } from '@shared/lib/editor-selection'
import { subscribeOpenFileFromEditor } from '@shared/lib/editor-opener-bridge'
import type { EditorPaneCommand, TabCycleDirection } from '@shared/lib/editor-pane-command-bridge'
import { subscribeEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { APP_KEYMAP, applyKeymapOverrides, parseKeymapOverrides } from '@shared/lib/keymap'
import { monaco } from '@shared/lib/monaco/setup'
import { collectPaneTabs, findPaneLeaf, findPaneTab } from '@shared/lib/pane-tree'
import { requestOpenSearchPanel } from '@shared/lib/search-panel-bridge'
import { requestTerminalWrite } from '@shared/lib/terminal-write-bridge'
import { TabItem } from '@features/tab/tab-item'
import type { TabContainerDropData } from '@widgets/editor-area/pane-tab-bar'
import { getTabIcon } from '@widgets/editor-area/pane-tab-bar'
import type { SplitDropData } from '@widgets/editor-area/pane-node-view'
import { PaneNodeView } from '@widgets/editor-area/pane-node-view'
import type { TabDragData } from '@widgets/editor-area/sortable-tab'
import { subscribeLanguageAdapterRegistration } from '@widgets/editor-pane/lsp-session-registry'
import { ProblemsPanelContainer } from '@widgets/problems-panel/problems-panel-container'

const DRAG_ACTIVATION_DISTANCE_PX = 4

const fileNameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

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
    const prunedProjectIdRef = useRef<ProjectId | null>(null)

    const [dragTab, setDragTab] = useState<DragTabState | null>(null)
    const [overTarget, setOverTarget] = useState<{ paneId: PaneId; edge: DropEdge } | null>(null)

    const { t } = useTranslation()
    const queryClient = useQueryClient()
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

    const getFocusedFileTabId = () => {
        if (!layout) return null
        const leaf = findPaneLeaf(layout.root, layout.focusedPane)
        const activeTab = leaf?.tabs.find((tab) => tab.id === leaf.active)
        return activeTab?.kind.kind === 'file' ? activeTab.id : null
    }

    const getFocusedFileEditor = () => {
        const tabId = getFocusedFileTabId()
        return tabId ? getEditorInstance(tabId) : null
    }

    const saveActiveTab = () => getFocusedFileEditor()?.getAction('taide.saveFile')?.run()

    const runMonacoAction = (actionId: string) => getFocusedFileEditor()?.trigger('taide.command', actionId, undefined)

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

    /**
     * Ensures the focused pane has a terminal tab (reusing one if present, otherwise opening a
     * new one) and writes `text` into its pty followed by a newline — the shared delivery path for
     * both "Run Selected Text in Terminal" and the task runner's "Run Task". Writes to a
     * freshly-opened tab race its pty spawn (`TerminalSession` measures/spawns asynchronously), so
     * they go through `terminal-write-bridge`'s queue-until-ready registration instead of `pty_write`
     * directly.
     */
    const runInTerminal = (text: string, cwd: string | null) => {
        if (!layout) return
        const leaf = findPaneLeaf(layout.root, layout.focusedPane)
        if (!leaf) return
        const payload = `${text}\n`

        const terminalTab = leaf.tabs.find((tab) => tab.kind.kind === 'terminal')
        if (terminalTab) {
            if (terminalTab.id !== leaf.active) activateTab(terminalTab.id)
            requestTerminalWrite(terminalTab.id, payload)
            return
        }

        openTab(
            { projectId, kind: { kind: 'terminal', sessionId: '', cwd }, title: t('terminal.title'), target: null, preview: false },
            {
                onSuccess: (nextLayout) => {
                    const nextActiveTabId = findPaneLeaf(nextLayout.root, nextLayout.focusedPane)?.active
                    if (nextActiveTabId) requestTerminalWrite(nextActiveTabId, payload)
                },
                onError: (error) => toast.error(error.message),
            },
        )
    }

    const runSelectedTextInTerminal = () => {
        const editor = getFocusedFileEditor()
        if (!editor) return
        const text = resolveSelectedTextOrCurrentLine(editor)
        if (text !== null) runInTerminal(text, null)
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
        if (command.type === 'run-monaco-action') return runMonacoAction(command.actionId)
        if (command.type === 'run-selected-text-in-terminal') return runSelectedTextInTerminal()
        if (command.type === 'run-in-terminal') return runInTerminal(command.text, command.cwd)
    })

    useEffect(() => subscribeEditorPaneCommand(handleEditorPaneCommand), [])

    /**
     * Consumes the cross-file navigation requests `registerLspEditorOpener` (app bootstrap) emits
     * when monaco needs to open a resource outside the current model — go-to-definition/
     * implementation/type-definition/declaration/references/F8 landing on another file. Mirrors
     * `ProblemsPanelContainer.handleOpenProblem`'s reveal-then-open pattern exactly.
     */
    const handleOpenFileFromEditor = useEffectEvent(({ path: targetPath, line, column }: { path: string; line: number; column: number }) => {
        requestReveal(targetPath, line, column)
        openTab(
            { projectId, kind: { kind: 'file', path: targetPath }, title: fileNameOf(targetPath), target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
        )
    })

    useEffect(() => subscribeOpenFileFromEditor(handleOpenFileFromEditor), [])

    const focusedFileTabId = getFocusedFileTabId()

    useEffect(() => {
        if (!focusedFileTabId) {
            setActiveEditorActionIds(null)
            return
        }

        let modelSubscription: { dispose: () => void } | null = null

        const updateActionIds = () => {
            const activeEditor = getEditorInstance(focusedFileTabId)
            setActiveEditorActionIds(activeEditor ? new Set(activeEditor.getSupportedActions().map((action) => action.id)) : null)
        }

        const attachToEditor = () => {
            modelSubscription?.dispose()
            const activeEditor = getEditorInstance(focusedFileTabId)
            modelSubscription = activeEditor?.onDidChangeModel(updateActionIds) ?? null
            updateActionIds()
        }

        attachToEditor()
        const editorSubscription = subscribeEditorInstance(focusedFileTabId, attachToEditor)
        const languageAdapterSubscription = subscribeLanguageAdapterRegistration(updateActionIds)

        return () => {
            editorSubscription()
            languageAdapterSubscription()
            modelSubscription?.dispose()
        }
    }, [focusedFileTabId])

    useEffect(() => () => setActiveEditorActionIds(null), [])

    /**
     * GC sweep for hot-exit mirrors, run once per project activation (guarded by
     * `prunedProjectIdRef` so later layout revisions in the same project don't re-trigger it).
     * File-path mirrors are kept for currently open tabs only — closing a file tab already clears
     * its mirror eagerly (`useCloseTab`'s `onSuccess`), so this is just a safety net for mirrors
     * left over from a session predating this feature or a crash.
     *
     * Untitled-tab mirrors are kept for open tabs *and* the closed-tab reopen stack (matching
     * `pane-tab-bar.tsx`'s own `pruneUntitledContents` keep set), since reopening a closed untitled
     * tab should still restore its draft. This is also the *authoritative* sweep for untitled
     * mirrors: it reads the restored layout directly rather than the frontend's in-memory
     * `untitled-registry`, which starts empty after every restart and so can't drive
     * `pruneUntitledContents` on its own.
     */
    useEffect(() => {
        if (!layout || prunedProjectIdRef.current === projectId) return
        prunedProjectIdRef.current = projectId

        const openTabs = collectPaneTabs(layout.root)
        const closedTabs = (layout.closedTabs ?? []).map((closed) => closed.tab)
        const keepPaths = openTabs.flatMap((tab) => (tab.kind.kind === 'file' ? [tab.kind.path] : []))
        const keepTabIds = [...openTabs, ...closedTabs].flatMap((tab) => (tab.kind.kind === 'untitled' ? [tab.id] : []))

        void pruneMirrors({ projectId, keepPaths })
            .then(() => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) }))
            .catch(() => undefined)
        void pruneUntitledMirrors({ projectId, keepTabIds })
            .then(() => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId) }))
            .catch(() => undefined)
    }, [projectId, layout, queryClient])

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
