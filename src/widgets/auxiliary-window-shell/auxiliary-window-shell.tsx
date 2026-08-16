import type { FC } from 'react'
import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ProjectId } from '@shared/api/bindings'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { IS_MAC } from '@shared/constants/platform'
import { isPaneTreeEmpty, resolveWindowPaneTree } from '@shared/lib/pane-tree'
import { EditorArea } from '@widgets/editor-area/editor-area'
import { AuxiliaryTitleBarContent } from '@widgets/auxiliary-window-shell/auxiliary-title-bar-content'

type AuxiliaryWindowShellProps = {
    projectId: ProjectId
    windowSlot: number
}

const noop = () => {}

/**
 * Editor-only chrome for an auxiliary window (`editor-<n>`) — no sidebar, no status bar, no
 * explorer, per contract §3.1's "사이드바·상태바 없는 에디터 전용 크롬". `EditorArea` resolves this
 * window's own `(projectId, windowSlot)` pane tree itself (`resolveWindowPaneTree`/
 * `window-context.ts`), so this component only threads `projectId` through and owns the chrome
 * around it.
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
    const { data: layout, isError } = useQuery(layoutQueryOptions(projectId))
    const paneTree = layout ? resolveWindowPaneTree(layout, { kind: 'auxiliary', projectId, windowSlot }) : null

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
                <EditorArea projectId={projectId} isProblemsOpen={false} onCloseProblems={noop} />
            </main>
        </div>
    )
}
