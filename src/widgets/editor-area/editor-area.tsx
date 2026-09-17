import type { FC } from 'react'
import { useEffect, useEffectEvent, useId, useRef, useState } from 'react'
import { DndContext, DragOverlay, PointerSensor, pointerWithin, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/core'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import type { DropEdge, PaneId, ProjectId, TabId, TabKind, TabWindowTarget } from '@shared/api/bindings'
import { getEditorInstance, subscribeEditorInstance } from '@entities/editor/editor-instance-registry'
import { pruneMirrors, pruneUntitledMirrors } from '@entities/file/file.ipc'
import {
    layoutQueryOptions,
    useActivateTab,
    useCloseTab,
    useFocusPane,
    useMoveTab,
    useMoveTabToWindow,
    useOpenFileTab,
    useOpenTab,
    useSplitPane,
} from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { setActiveEditorActionIds } from '@shared/lib/bridge/active-editor-actions-bridge'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { QUERY_KEY } from '@shared/constants/query-key'
import { resolveSelectedTextOrCurrentLine } from '@shared/lib/editor-selection'
import { subscribeOpenFileFromEditor } from '@shared/lib/bridge/editor-opener-bridge'
import type { EditorPaneCommand, PaneFocusTarget, TabCycleDirection } from '@shared/lib/bridge/editor-pane-command-bridge'
import { subscribeEditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { monaco } from '@shared/lib/monaco/setup'
import type { PaneDirection } from '@shared/lib/pane-tree'
import { collectAllPaneTabs, findAdjacentPaneLeaf, findPaneLeaf, findPaneTab, resolveWindowPaneTree } from '@shared/lib/pane-tree'
import { requestOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { requestTerminalWrite } from '@shared/lib/bridge/terminal-write-bridge'
import { useIsShellSlotFocused } from '@shared/lib/shell-slot-context'
import { getWindowContext } from '@shared/lib/window-context'
import { TabItem } from '@features/tab/tab-item'
import type { TabContainerDropData } from '@widgets/editor-area/pane-tab-bar'
import { getTabIcon } from '@widgets/editor-area/pane-tab-bar'
import { collectClosableTabIdsInFocusedGroup, resolveFocusGroupPaneId } from '@widgets/editor-area/group-shortcut-targets'
import type { SplitDropData } from '@widgets/editor-area/pane-node-view'
import { PaneNodeView } from '@widgets/editor-area/pane-node-view'
import { resolveSaveRoutableTabId } from '@widgets/editor-area/focused-editor-tab'
import type { TabDragData } from '@features/tab/sortable-tab'
import { subscribeLanguageAdapterRegistration } from '@entities/lsp/lsp-session-registry'
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
    /**
     * Window-level as of d-62 §0.1 S-6 — it used to be read here off this project's `shell_view`,
     * which stopped being the truth once one window could hold several project shells. An auxiliary
     * window passes `false`: it is editor-only chrome with nothing to hide, and the session-wide Zen
     * flag must not reach it just because the main window happens to be in Zen mode.
     */
    zen: boolean
    isProblemsOpen: boolean
    onCloseProblems: () => void
}

/**
 * Every global listener below is gated on {@link useIsShellSlotFocused} (contract §0.1 S-4/U-1): this
 * component is mounted once per open shell slot, and both `useGlobalKeymap` and the two broadcast
 * bridges it subscribes to are per-realm rather than per-slot — ungated, one ⌘S or one palette
 * "Split Editor" would fire once for every slot on screen. Outside a slot scope (an auxiliary window,
 * a component test) the gate is always open, so those realms behave exactly as before.
 */
export const EditorArea: FC<EditorAreaProps> = ({ projectId, zen, isProblemsOpen, onCloseProblems }) => {
    const prunedProjectIdRef = useRef<ProjectId | null>(null)

    /** One of these mounts per shell slot (d-62 §1.B), and `Panel` publishes its `id` as the DOM `id` its separator's `aria-controls` points at — so the two panes below are namespaced per instance rather than being duplicate ids in one document (`project-shell.tsx` carries the full note). */
    const panelIdPrefix = useId()

    const [dragTab, setDragTab] = useState<DragTabState | null>(null)
    const [overTarget, setOverTarget] = useState<{ paneId: PaneId; edge: DropEdge } | null>(null)

    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: moveTab } = useMoveTab(projectId)
    const { mutate: splitPane } = useSplitPane(projectId)
    const { mutate: closeTab, mutateAsync: closeTabAsync } = useCloseTab(projectId)
    const { mutate: activateTab } = useActivateTab(projectId)
    const { mutate: openTab } = useOpenTab(projectId)
    const openFileTab = useOpenFileTab()
    const { mutate: moveTabToWindow } = useMoveTabToWindow(projectId)
    const { mutate: focusPane } = useFocusPane(projectId)
    const isFocused = useIsShellSlotFocused()

    /**
     * Which of the project's pane trees *this* window renders — the main tree for the main window,
     * or this window's own `AuxWindowLayout` entry for an auxiliary window (Wave I contract §3.1).
     * Every place below that used to read `layout.root`/`layout.focusedPane` directly now reads
     * `paneTree` instead, so the exact same keymap/DnD/render logic works unmodified for whichever
     * tree this window owns — see `resolveWindowPaneTree`'s doc comment for the `null` case.
     */
    const windowContext = getWindowContext()
    const paneTree = layout ? resolveWindowPaneTree(layout, windowContext) : null

    /** A pinned tab survives ⌘W with a warning instead of closing (`docs/features/tabs.md` §3) — the tab bar's own close affordances guard themselves in `tab-item.tsx`. */
    const closeFocusedTab = () => {
        if (!paneTree) return
        const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
        if (!leaf?.active) return
        const activeTab = leaf.tabs.find((tab) => tab.id === leaf.active)
        if (activeTab?.pinned) {
            toast.warning(t('tab.pinnedCloseBlocked', { title: activeTab.title }))
            return
        }
        closeTab(leaf.active)
    }

    const moveFocusedTabToWindow = (target: TabWindowTarget) => {
        if (!paneTree) return
        const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
        if (!leaf?.active) return
        moveTabToWindow({ tabId: leaf.active, target }, { onError: (error) => toast.error(describeIpcError(error)) })
    }

    /**
     * `hasWidgetFocus`, not `hasTextFocus`: the latter is true only while the editor's own
     * `textarea.inputarea` holds focus, so pressing ⌘F *again* while the Find widget's input is
     * focused (the natural "search the next thing" reflex) read as "no editor focused" and opened
     * the global search panel instead of re-triggering monaco's find — leaving the editor's find
     * widget behind. Widget focus covers the editor container including its overlay widgets (find,
     * rename, ...), which is exactly the surface ⌘F belongs to.
     */
    const openFind = () => {
        const focusedEditor = monaco.editor.getEditors().find((instance) => instance.hasWidgetFocus())
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
        if (!paneTree) return
        const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
        if (!leaf?.active) return
        splitPane({ paneId: paneTree.focusedPane, edge: 'right', tabId: leaf.active })
    }

    const cycleTab = (direction: TabCycleDirection) => {
        if (!paneTree) return
        const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
        if (!leaf?.active) return
        if (leaf.tabs.length < 2) return
        const currentIndex = leaf.tabs.findIndex((tab) => tab.id === leaf.active)
        if (currentIndex < 0) return
        const step = direction === 'next' ? 1 : -1
        const nextIndex = (currentIndex + step + leaf.tabs.length) % leaf.tabs.length
        activateTab(leaf.tabs[nextIndex].id)
    }

    /** ⌘K ⌘←/→/↑/↓ and ⌘1..⌘9 both land here; {@link resolveFocusGroupPaneId} owns which group that is and which presses are no-ops. */
    const focusGroup = (target: PaneFocusTarget) => {
        const paneId = resolveFocusGroupPaneId(paneTree, target)
        if (!paneId) return
        focusPane(paneId)
    }

    /** Appends the focused tab to the neighbouring group's strip; `layout_move_tab` focuses the destination pane itself, so the tab stays with the user. */
    const moveActiveTabToGroup = (direction: PaneDirection) => {
        if (!paneTree) return
        const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
        if (!leaf?.active) return
        const adjacent = findAdjacentPaneLeaf(paneTree.root, paneTree.focusedPane, direction)
        if (!adjacent) return
        moveTab({ tabId: leaf.active, paneId: adjacent.id, index: adjacent.tabs.length })
    }

    /** Closes are serialized because each one returns a fresh layout the next close would have to be computed against; {@link collectClosableTabIdsInFocusedGroup} owns the pinned-survives rule. */
    const closeAllTabsInFocusedGroup = async () => {
        for (const tabId of collectClosableTabIdsInFocusedGroup(paneTree)) {
            await closeTabAsync(tabId)
        }
    }

    const getFocusedSaveRoutableTabId = () => {
        if (!paneTree) return null
        return resolveSaveRoutableTabId(findPaneLeaf(paneTree.root, paneTree.focusedPane))
    }

    const getFocusedSaveRoutableEditor = () => {
        const tabId = getFocusedSaveRoutableTabId()
        return tabId ? getEditorInstance(tabId) : null
    }

    const saveActiveTab = () => getFocusedSaveRoutableEditor()?.getAction('taide.saveFile')?.run()

    const runMonacoAction = (actionId: string) => getFocusedSaveRoutableEditor()?.trigger('taide.command', actionId, undefined)

    const toggleTerminal = () => {
        if (!paneTree) return
        const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
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
            { projectId, kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: paneTree.focusedPane, preview: false },
            { onError: (error) => toast.error(describeIpcError(error)) },
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
        if (!paneTree) return
        const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
        if (!leaf) return
        const payload = `${text}\n`

        const terminalTab = leaf.tabs.find((tab) => tab.kind.kind === 'terminal')
        if (terminalTab) {
            if (terminalTab.id !== leaf.active) activateTab(terminalTab.id)
            requestTerminalWrite(terminalTab.id, payload)
            return
        }

        openTab(
            { projectId, kind: { kind: 'terminal', sessionId: '', cwd }, title: t('terminal.title'), target: paneTree.focusedPane, preview: false },
            {
                onSuccess: (nextLayout) => {
                    const nextPaneTree = resolveWindowPaneTree(nextLayout, windowContext)
                    const nextActiveTabId = nextPaneTree ? findPaneLeaf(nextPaneTree.root, nextPaneTree.focusedPane)?.active : null
                    if (nextActiveTabId) requestTerminalWrite(nextActiveTabId, payload)
                },
                onError: (error) => toast.error(describeIpcError(error)),
            },
        )
    }

    const runSelectedTextInTerminal = () => {
        const editor = getFocusedSaveRoutableEditor()
        if (!editor) return
        const text = resolveSelectedTextOrCurrentLine(editor)
        if (text !== null) runInTerminal(text, null)
    }

    /** `undefined` rather than a no-op function: `useGlobalKeymap` only calls `preventDefault` when a handler exists, so an unfocused slot leaves the keystroke for the focused one to claim. */
    const whenFocused = (handler: () => void) => (isFocused ? handler : undefined)

    useGlobalKeymap({
        'close-tab': whenFocused(closeFocusedTab),
        find: whenFocused(openFind),
        search: whenFocused(openGlobalSearch),
        'search-replace': whenFocused(() => requestOpenSearchPanel({ openReplace: true })),
        split: whenFocused(splitActiveEditor),
        'tab-cycle-next': whenFocused(() => cycleTab('next')),
        'tab-cycle-prev': whenFocused(() => cycleTab('prev')),
        'editor-next': whenFocused(() => cycleTab('next')),
        'editor-previous': whenFocused(() => cycleTab('prev')),
        save: whenFocused(saveActiveTab),
        'toggle-terminal': whenFocused(toggleTerminal),
        'focus-group-left': whenFocused(() => focusGroup({ kind: 'direction', direction: 'left' })),
        'focus-group-right': whenFocused(() => focusGroup({ kind: 'direction', direction: 'right' })),
        'focus-group-up': whenFocused(() => focusGroup({ kind: 'direction', direction: 'up' })),
        'focus-group-down': whenFocused(() => focusGroup({ kind: 'direction', direction: 'down' })),
        'move-tab-to-group-left': whenFocused(() => moveActiveTabToGroup('left')),
        'move-tab-to-group-right': whenFocused(() => moveActiveTabToGroup('right')),
        'close-all-tabs': whenFocused(() => void closeAllTabsInFocusedGroup()),
        'focus-group-1': whenFocused(() => focusGroup({ kind: 'position', position: 1 })),
        'focus-group-2': whenFocused(() => focusGroup({ kind: 'position', position: 2 })),
        'focus-group-3': whenFocused(() => focusGroup({ kind: 'position', position: 3 })),
        'focus-group-4': whenFocused(() => focusGroup({ kind: 'position', position: 4 })),
        'focus-group-5': whenFocused(() => focusGroup({ kind: 'position', position: 5 })),
        'focus-group-6': whenFocused(() => focusGroup({ kind: 'position', position: 6 })),
        'focus-group-7': whenFocused(() => focusGroup({ kind: 'position', position: 7 })),
        'focus-group-8': whenFocused(() => focusGroup({ kind: 'position', position: 8 })),
        'focus-group-9': whenFocused(() => focusGroup({ kind: 'position', position: 9 })),
    })

    const handleEditorPaneCommand = useEffectEvent((command: EditorPaneCommand) => {
        if (!isFocused) return
        if (command.type === 'split') return splitActiveEditor()
        if (command.type === 'cycle-tab') return cycleTab(command.direction)
        if (command.type === 'save-active-tab') return saveActiveTab()
        if (command.type === 'toggle-terminal') return toggleTerminal()
        if (command.type === 'run-monaco-action') return runMonacoAction(command.actionId)
        if (command.type === 'run-selected-text-in-terminal') return runSelectedTextInTerminal()
        if (command.type === 'run-in-terminal') return runInTerminal(command.text, command.cwd)
        if (command.type === 'move-focused-tab-to-window') return moveFocusedTabToWindow(command.target)
        if (command.type === 'focus-group') return focusGroup(command.target)
        if (command.type === 'move-tab-to-group') return moveActiveTabToGroup(command.direction)
        if (command.type === 'close-all-tabs') return void closeAllTabsInFocusedGroup()
    })

    useEffect(() => subscribeEditorPaneCommand(handleEditorPaneCommand), [])

    /**
     * Consumes the cross-file navigation requests `registerLspEditorOpener` (app bootstrap) emits
     * when monaco needs to open a resource outside the current model — go-to-definition/
     * implementation/type-definition/declaration/references/F8 landing on another file. Mirrors
     * `ProblemsPanelContainer.handleOpenProblem` exactly: the jump rides along with the open as
     * `reveal`, so it lands on the tab THIS open produced rather than on whichever pane already had
     * the file (audit #9).
     */
    const handleOpenFileFromEditor = useEffectEvent(({ path: targetPath, line, column }: { path: string; line: number; column: number }) => {
        if (!isFocused) return
        openFileTab({ projectId, path: targetPath, target: paneTree?.focusedPane ?? null, preview: true, reveal: { line, column } })
    })

    useEffect(() => subscribeOpenFileFromEditor(handleOpenFileFromEditor), [])

    const focusedSaveRoutableTabId = getFocusedSaveRoutableTabId()

    useEffect(() => {
        if (!isFocused) return
        if (!focusedSaveRoutableTabId) {
            setActiveEditorActionIds(null)
            return
        }

        let modelSubscription: { dispose: () => void } | null = null

        const updateActionIds = () => {
            const activeEditor = getEditorInstance(focusedSaveRoutableTabId)
            if (!activeEditor) {
                setActiveEditorActionIds(null)
                return
            }
            /**
             * `editor.addAction`-registered actions (TAIDE's own `taide.*` catalog entries) report
             * `action.id` as `${editor.getId()}:${originalId}` — monaco mangles the id into a
             * per-editor-instance "unique id" internally (`standaloneCodeEditor.js`'s `addAction`)
             * so the same action id can be registered on multiple editor instances at once, but
             * every id-keyed consumer of this set (`monaco-action-commands.ts`'s `isEnabled` gate,
             * the keybindings editor's context checks) only knows the original, unprefixed id monaco
             * built-in actions report unchanged. Stripping this editor's own prefix restores that —
             * built-in action ids never start with it, so they pass through untouched.
             */
            const uniqueIdPrefix = `${activeEditor.getId()}:`
            const ids = activeEditor
                .getSupportedActions()
                .map((action) => (action.id.startsWith(uniqueIdPrefix) ? action.id.slice(uniqueIdPrefix.length) : action.id))
            setActiveEditorActionIds(new Set(ids))
        }

        const attachToEditor = () => {
            modelSubscription?.dispose()
            const activeEditor = getEditorInstance(focusedSaveRoutableTabId)
            modelSubscription = activeEditor?.onDidChangeModel(updateActionIds) ?? null
            updateActionIds()
        }

        attachToEditor()
        const editorSubscription = subscribeEditorInstance(focusedSaveRoutableTabId, attachToEditor)
        const languageAdapterSubscription = subscribeLanguageAdapterRegistration(updateActionIds)

        return () => {
            editorSubscription()
            languageAdapterSubscription()
            modelSubscription?.dispose()
        }
    }, [isFocused, focusedSaveRoutableTabId])

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
     *
     * Reads `collectAllPaneTabs` (main tree *and* every auxiliary window's tree), not just this
     * window's own `paneTree` — both the main and every auxiliary window mount their own
     * `EditorArea` and independently run this same sweep, so a window scoping it to only its own
     * tabs would prune mirrors for files merely open in a *different* window.
     */
    useEffect(() => {
        if (!layout || prunedProjectIdRef.current === projectId) return
        prunedProjectIdRef.current = projectId

        const openTabs = collectAllPaneTabs(layout)
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
        if (!paneTree) return
        const tab = findPaneTab(paneTree.root, active.id as TabId)
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
        if (!over || !paneTree) return

        const tabId = active.id as TabId
        const activeData = active.data.current as TabDragData | undefined
        const overData = over.data.current as OverDropData | TabDragData | undefined
        if (!activeData || !overData) return

        if (overData.type === 'split') {
            if (overData.edge === 'center') {
                const leaf = findPaneLeaf(paneTree.root, overData.paneId)
                if (leaf) moveTab({ tabId, paneId: overData.paneId, index: leaf.tabs.length })
                return
            }
            splitPane({ paneId: overData.paneId, edge: overData.edge, tabId })
            return
        }

        if (overData.type === 'tab-container') {
            const leaf = findPaneLeaf(paneTree.root, overData.paneId)
            if (leaf) moveTab({ tabId, paneId: overData.paneId, index: leaf.tabs.length })
            return
        }

        const leaf = findPaneLeaf(paneTree.root, overData.paneId)
        if (!leaf) return
        const rawIndex = leaf.tabs.findIndex((tab) => tab.id === over.id)
        if (rawIndex < 0) return
        const pinnedCount = leaf.tabs.filter((tab) => tab.pinned).length
        const index = activeData.pinned ? Math.min(rawIndex, pinnedCount) : Math.max(rawIndex, pinnedCount)
        moveTab({ tabId, paneId: overData.paneId, index })
    }

    if (!layout || !paneTree) return <div className='bg-editor-background h-full w-full' />

    return (
        <DndContext
            sensors={sensors}
            collisionDetection={pointerWithin}
            onDragStart={handleDragStart}
            onDragOver={handleDragOver}
            onDragEnd={handleDragEnd}
            onDragCancel={handleDragCancel}>
            <Group orientation='vertical' className='min-h-0 min-w-0 flex-1'>
                <Panel id={`${panelIdPrefix}-editor-panes`} minSize='30%' className='min-h-0 min-w-0'>
                    <div className='relative flex h-full min-h-0 w-full min-w-0 overflow-hidden'>
                        <PaneNodeView
                            node={paneTree.root}
                            projectId={projectId}
                            focusedPaneId={paneTree.focusedPane}
                            isDragging={!!dragTab}
                            overTarget={overTarget}
                            zen={zen}
                        />
                    </div>
                </Panel>
                {isProblemsOpen && <PaneSeparator orientation='vertical' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />}
                {isProblemsOpen && (
                    <Panel id={`${panelIdPrefix}-problems-panel`} defaultSize='220px' minSize='120px' className='min-h-0 min-w-0'>
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
                            onTogglePin={() => {}}
                        />
                    </div>
                )}
            </DragOverlay>
        </DndContext>
    )
}
