import type { FC } from 'react'
import { useEffect, useEffectEvent, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Group, Panel, usePanelRef } from 'react-resizable-panels'
import type { ProjectId } from '@shared/api/bindings'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { DEFAULT_RESIZER_THICKNESS, RESIZE_HIT_TARGET_SIZE } from '@shared/constants/layout'
import { IS_MAC } from '@shared/constants/platform'
import {
    requestShowExplorerView,
    requestToggleExplorerSidebar,
    subscribeShowExplorerView,
    subscribeToggleExplorerSidebar,
} from '@shared/lib/bridge/explorer-panel-bridge'
import { subscribeRevealInExplorer } from '@shared/lib/bridge/explorer-reveal-bridge'
import { subscribeRenameInExplorer } from '@shared/lib/bridge/explorer-rename-bridge'
import { subscribeOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { isPaneTreeEmpty, resolveWindowPaneTree } from '@shared/lib/pane-tree'
import { ErrorBoundary } from '@shared/ui/error-boundary'
import { EditorArea } from '@widgets/editor-area/editor-area'
import { ExplorerContainer } from '@widgets/explorer/explorer-container'
import { AuxiliaryTitleBarContent } from '@widgets/auxiliary-window-shell/auxiliary-title-bar-content'

type AuxiliaryWindowShellProps = {
    projectId: ProjectId
    windowSlot: number
}

const noop = () => {}

/**
 * Chrome for an auxiliary window (`editor-<n>`): the explorer panel plus the editor area, and
 * nothing else — no app sidebar (an auxiliary window is pinned to one project, so a project switcher
 * would have nothing to switch) and no status bar, per contract §3.1's "사이드바·상태바 없는 에디터
 * 전용 크롬". The panel body itself is `ExplorerContainer`, which brings the files/search/git/outline
 * switcher with it (`explorer-panel.tsx`), so all three of d-62 §1.D's containers reach this window
 * through one mount rather than three parallel ones.
 *
 * Every bridge this wires (`explorer-panel`/`explorer-reveal`/`explorer-rename`/`search-panel`) is
 * module state inside one JS realm, and Tauri gives every window its own webview and therefore its
 * own realm — so a ⌘B or a "Reveal in Explorer" published here reaches only this window's panel, and
 * the main window's `AppShell` (which subscribes to the same bridges in its own realm) never hears
 * it. That per-realm isolation is what lets this file reuse `AppShell`'s wiring verbatim instead of
 * needing a window-scoped channel.
 *
 * The collapsed/expanded state is deliberately view-local here, unlike `ProjectShell`'s, which
 * persists to Rust-owned `shell_view.sidebarCollapsed`: that field is keyed by project, not by
 * window, so a main window and an auxiliary window on the same project would overwrite each other's
 * panel every time either one toggled. d-62 §0.1 S-6 moved Zen and the icon rail to the *session*
 * instead, which is why `zen={false}` is passed literally below — a session-wide flag the main window
 * raised must not hide this window's chrome.
 *
 * `EditorArea` resolves this window's own `(projectId, windowSlot)` pane tree itself
 * (`resolveWindowPaneTree`/`window-context.ts`), so this component only threads `projectId` through.
 *
 * Also closes this OS window once its own tree goes empty — the *close* half of contract §3.2's
 * "마지막 탭 이동/닫기 시 창 정리". `layout_move_tab_to_window` already closes this window
 * server-side when its last tab *moves* elsewhere (`cleanup_emptied_auxiliary_windows`), but a plain
 * tab close (✕/⌘W) deliberately leaves the window open server-side (S2's documented minimal-scope
 * decision) — watching the resolved tree here is the frontend half that completes the symmetry.
 *
 * Also closes on `isError` (not just `!layout`), because `useQuery` keeps serving the last
 * successful `layout` value while a later refetch fails — `layout_get` only ever errors with
 * `NotFound` (`domain::layout::commands::layout_get`), which happens when the main window closes
 * this project (`project_close` removes the project's `state.layouts` entry) while this auxiliary
 * window is still open on it. Without this branch the window would freeze on its last-known tree
 * forever, since the server has nothing left to serve and no further `LayoutChanged` will arrive.
 */
export const AuxiliaryWindowShell: FC<AuxiliaryWindowShellProps> = ({ projectId, windowSlot }) => {
    const explorerPanelRef = usePanelRef()

    /**
     * This window's own panel state. There is no persisted field to read it from — the collapse is
     * deliberately view-local here (see above) — and the explorer's auto-reveal gate needs it, so it
     * is mirrored into state from `onLayoutChanged`, which `react-resizable-panels` fires for the
     * imperative `.collapse()`/`.expand()` calls below as well as for drags (`LayoutChangedMeta`).
     */
    const [explorerPanelCollapsed, setExplorerPanelCollapsed] = useState(false)

    const { data: layout, isError } = useQuery(layoutQueryOptions(projectId))
    const { data: settings } = useQuery(settingsQueryOptions())
    const paneTree = layout ? resolveWindowPaneTree(layout, { kind: 'auxiliary', projectId, windowSlot }) : null

    /** `useEffectEvent` (not a dependency array) so the bridge is subscribed exactly once while the handler still reads the panel's latest imperative state — the same shape `AppShell` uses. */
    const handleToggleSidebarRequested = useEffectEvent(() => {
        const panel = explorerPanelRef.current
        if (!panel) return
        if (panel.isCollapsed()) panel.expand()
        else panel.collapse()
    })

    useGlobalKeymap({
        'toggle-sidebar': () => requestToggleExplorerSidebar(),
        explorer: () => requestShowExplorerView('files'),
        git: () => requestShowExplorerView('git'),
    })

    useEffect(() => subscribeToggleExplorerSidebar(handleToggleSidebarRequested), [])

    /** Each of these is an explicit request for the panel, so it expands a collapsed one first — `AppShell` expands on the same four bridges and for the same reason. */
    useEffect(() => subscribeOpenSearchPanel(() => explorerPanelRef.current?.expand()), [explorerPanelRef])
    useEffect(() => subscribeShowExplorerView(() => explorerPanelRef.current?.expand()), [explorerPanelRef])
    useEffect(() => subscribeRevealInExplorer(() => explorerPanelRef.current?.expand()), [explorerPanelRef])
    useEffect(() => subscribeRenameInExplorer(() => explorerPanelRef.current?.expand()), [explorerPanelRef])

    useEffect(() => {
        if (!isError && !layout) return
        if (!isError && paneTree && !isPaneTreeEmpty(paneTree.root)) return
        void getCurrentWindow()
            .close()
            .catch(() => undefined)
    }, [layout, paneTree, isError])

    return (
        <div className='bg-app-background text-app-foreground relative flex h-full w-full flex-col'>
            {IS_MAC && (
                <div className='border-tab-bar-tab-border shrink-0 border-b'>
                    <AuxiliaryTitleBarContent projectId={projectId} windowSlot={windowSlot} />
                </div>
            )}
            <main className='flex min-h-0 min-w-0 flex-1'>
                <Group
                    orientation='horizontal'
                    onLayoutChanged={() => setExplorerPanelCollapsed(explorerPanelRef.current?.isCollapsed() ?? false)}
                    resizeTargetMinimumSize={RESIZE_HIT_TARGET_SIZE}
                    className='min-h-0 min-w-0 flex-1'>
                    <Panel id='explorer' panelRef={explorerPanelRef} defaultSize='240px' minSize='180px' maxSize='40%' collapsible collapsedSize={0}>
                        <ErrorBoundary labelKey='errorBoundary.sidebarPanel' labelFallback='Sidebar Panel'>
                            <ExplorerContainer projectId={projectId} zen={false} sidebarCollapsed={explorerPanelCollapsed} />
                        </ErrorBoundary>
                    </Panel>
                    <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                    <Panel id='editor' minSize='30%'>
                        <ErrorBoundary labelKey='errorBoundary.editorArea' labelFallback='Editor'>
                            <EditorArea projectId={projectId} zen={false} isProblemsOpen={false} onCloseProblems={noop} />
                        </ErrorBoundary>
                    </Panel>
                </Group>
            </main>
        </div>
    )
}
