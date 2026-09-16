import type { FC } from 'react'
import { useEffect, useEffectEvent, useId } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Group, Panel, usePanelRef } from 'react-resizable-panels'
import type { Layout, LayoutChangedMeta } from 'react-resizable-panels'
import type { ProjectId } from '@shared/api/bindings'
import { layoutQueryOptions, useSetShellView } from '@entities/layout/layout.query'
import { schedulePaneResizeCommit } from '@entities/layout/pane-resize-commit'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { DEFAULT_RESIZER_THICKNESS, RESIZE_HIT_TARGET_SIZE } from '@shared/constants/layout'
import { subscribeToggleExplorerSidebar, subscribeShowExplorerView } from '@shared/lib/bridge/explorer-panel-bridge'
import { subscribeRevealInExplorer } from '@shared/lib/bridge/explorer-reveal-bridge'
import { subscribeRenameInExplorer } from '@shared/lib/bridge/explorer-rename-bridge'
import { subscribeOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { useIsShellSlotFocused } from '@shared/lib/shell-slot-context'
import { ErrorBoundary } from '@shared/ui/error-boundary'
import { EditorArea } from '@widgets/editor-area/editor-area'
import { ExplorerContainer } from '@widgets/explorer/explorer-container'

type ProjectShellProps = {
    projectId: ProjectId
    zen: boolean
    isProblemsOpen: boolean
    onCloseProblems: () => void
}

/**
 * One project's whole shell — explorer panel, editor area, problems panel — as it used to sit inside
 * `AppShell`, now a component so the window can render several of them side by side (contract §1.B).
 * Every project-scoped thing below it was already `projectId`-driven, which is what made the split
 * possible without touching the panels themselves.
 *
 * The four panel bridges are gated on `useIsShellSlotFocused`: they are per-realm broadcasts with no
 * slot address of their own (contract §0.1 U-1), so without the gate one ⌘B would toggle every open
 * slot's explorer at once. The publishers stay untouched — the focused slot is the only one that
 * should answer, and that is exactly what the gate expresses. Auxiliary windows have no slot scope at
 * all, so their copy of every widget below keeps behaving as before.
 */
export const ProjectShell: FC<ProjectShellProps> = ({ projectId, zen, isProblemsOpen, onCloseProblems }) => {
    const explorerPanelRef = usePanelRef()

    /**
     * `Panel` publishes its `id` verbatim as the DOM `id`, and the separator between two panels points
     * at those ids through `aria-controls`. A window now holds one of these shells per open slot
     * (d-62 §1.B), so static ids would put duplicates in one document and aim every slot's separator
     * at the *first* slot's panels. The per-instance prefix is what keeps each shell's pair its own.
     * `auxiliary-window-shell` is a separate document and keeps the plain ids.
     */
    const panelIdPrefix = useId()

    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: setShellView } = useSetShellView(projectId)
    const isFocused = useIsShellSlotFocused()

    const sidebarCollapsed = layout?.shellView?.sidebarCollapsed ?? false

    /** The explorer panel's collapsed state stays a *slot-local* axis on the per-project layout — only Zen and the icon rail moved to the window (contract §0.1 S-6). The panel's *width* remains a view-local default. */
    const persistSidebarCollapsed = (collapsed: boolean) => setShellView({ projectId, patch: { zen: null, sidebarCollapsed: collapsed } })

    /**
     * Catches a *drag*-driven collapse/expand (dragging the separator past `minSize`) — imperative
     * `.collapse()`/`.expand()` calls report `isUserInteraction: false` and persist explicitly at their
     * own call site instead.
     *
     * Only a *crossing* is worth a write: a resize that leaves the panel on the same side of `minSize`
     * would persist the value already stored. That matters because this notification is not one per
     * gesture — `react-resizable-panels` reports keyboard resizing once per keydown, auto-repeat
     * included, so a held arrow key fired one `layout_set_shell_view` per repeat. The surviving
     * crossings go through the same trailing debounce as every other pane resize
     * (`pane-resize-commit.ts`), since one drag can cross `minSize` several times before it rests.
     */
    const handleShellLayoutChanged = (_layout: Layout, meta: LayoutChangedMeta) => {
        if (!meta.isUserInteraction) return
        const collapsed = explorerPanelRef.current?.isCollapsed() ?? false
        if (collapsed === sidebarCollapsed) return
        schedulePaneResizeCommit(`${projectId}:sidebar-collapsed`, () => persistSidebarCollapsed(collapsed))
    }

    /**
     * Search, view switch, "Reveal in Explorer" and a tab's "Rename" are all explicit requests for the
     * panel, so each expands a collapsed one first — otherwise the tab context menu's entries looked
     * like no-ops whenever the sidebar was collapsed. Auto-reveal deliberately does not go through
     * these bridges, so it can never pop the panel open on its own.
     */
    const handlePanelExpandRequested = useEffectEvent(() => {
        if (!isFocused) return
        explorerPanelRef.current?.expand()
    })

    /**
     * A no-op while Zen mode holds the panel force-collapsed — toggling it mid-Zen would desync
     * from the `shouldCollapse` sync effect below (no separator to drag it back with either, since
     * that's also hidden in Zen), so the manual toggle is suppressed until the user leaves Zen
     * mode. `useEffectEvent` (not a dependency array) so the effect subscribes exactly once while
     * still always reading the *latest* `zen`/`isFocused` at the moment the bridge fires.
     */
    const handleToggleSidebarRequested = useEffectEvent(() => {
        if (!isFocused || zen) return
        const panel = explorerPanelRef.current
        if (!panel) return
        const collapsed = panel.isCollapsed()
        if (collapsed) panel.expand()
        else panel.collapse()
        persistSidebarCollapsed(!collapsed)
    })

    useEffect(() => subscribeOpenSearchPanel(handlePanelExpandRequested), [])
    useEffect(() => subscribeShowExplorerView(handlePanelExpandRequested), [])
    useEffect(() => subscribeRevealInExplorer(handlePanelExpandRequested), [])
    useEffect(() => subscribeRenameInExplorer(handlePanelExpandRequested), [])
    useEffect(() => subscribeToggleExplorerSidebar(handleToggleSidebarRequested), [])

    /** Applies Zen mode (always collapsed) and the persisted `sidebarCollapsed` preference (otherwise) to the panel's actual imperative state — an external-widget sync, not a derived render value, since `Panel` has no controlled "collapsed" prop. */
    useEffect(() => {
        const panel = explorerPanelRef.current
        if (!panel) return
        const shouldCollapse = zen || sidebarCollapsed
        if (panel.isCollapsed() === shouldCollapse) return
        if (shouldCollapse) panel.collapse()
        else panel.expand()
    }, [explorerPanelRef, zen, sidebarCollapsed])

    return (
        <Group
            orientation='horizontal'
            onLayoutChanged={handleShellLayoutChanged}
            resizeTargetMinimumSize={RESIZE_HIT_TARGET_SIZE}
            className='min-h-0 min-w-0 flex-1'>
            <Panel
                id={`${panelIdPrefix}-explorer`}
                panelRef={explorerPanelRef}
                defaultSize='240px'
                minSize='180px'
                maxSize='40%'
                collapsible
                collapsedSize={0}>
                <ErrorBoundary labelKey='errorBoundary.sidebarPanel' labelFallback='Sidebar Panel'>
                    <ExplorerContainer projectId={projectId} zen={zen} />
                </ErrorBoundary>
            </Panel>
            {!zen && <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />}
            <Panel id={`${panelIdPrefix}-editor`} minSize='30%'>
                <ErrorBoundary labelKey='errorBoundary.editorArea' labelFallback='Editor'>
                    <EditorArea projectId={projectId} zen={zen} isProblemsOpen={isProblemsOpen} onCloseProblems={onCloseProblems} />
                </ErrorBoundary>
            </Panel>
        </Group>
    )
}
