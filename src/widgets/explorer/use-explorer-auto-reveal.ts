import { useEffect, useEffectEvent, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import type { useRevealTreeNode } from '@entities/tree/tree.query'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { activeFilePathOf } from '@shared/lib/pane-tree'
import { toRelativePath } from '@shared/lib/relative-path'
import { decideAutoReveal } from '@widgets/explorer/explorer-auto-reveal'

type UseExplorerAutoRevealInput = {
    projectId: ProjectId
    projectRoot: string | null
    rows: FileTreeRow[]
    explorerViewActive: boolean
    setSelectPathRequest: (path: string) => void
    revealTreeNode: ReturnType<typeof useRevealTreeNode>['mutateAsync']
}

/**
 * Keeps the file tree pointed at the active editor tab (VS Code's `explorer.autoReveal`), reusing
 * the reveal path the tab context menu's "Reveal in Explorer" already drives: `tree_reveal` to load
 * and expand the ancestors, then the existing `selectPathRequest` handshake to scroll and select.
 * Selection never touches DOM focus, so typing in the editor is uninterrupted.
 *
 * The active file is read from the *main* pane tree only (`layout.root`/`layout.focusedPane`, not
 * `resolveWindowPaneTree`): the sidebar is mounted in the main window alone, so letting an
 * auxiliary window's focused tab steer it would move the tree under a user who is looking at a
 * different window.
 *
 * A path outside the project root can't be revealed — `tree_reveal` has no ancestors under the root
 * to walk — so it is filtered out here rather than being sent and failing. Everything else that
 * decides *whether* to act lives in `decideAutoReveal`; this hook only supplies the live inputs and
 * performs the chosen action. Reveal failures stay silent: the user did not ask for this, and a
 * toast for a background tree operation would be noise (the explicit menu path still reports its
 * own errors).
 */
export const useExplorerAutoReveal = ({
    projectId,
    projectRoot,
    rows,
    explorerViewActive,
    setSelectPathRequest,
    revealTreeNode,
}: UseExplorerAutoRevealInput) => {
    const lastRevealedRef = useRef<string | null>(null)

    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { data: settings } = useQuery(settingsQueryOptions())

    const activePath = activeFilePathOf(layout)
    const isUnderProjectRoot = !!activePath && !!projectRoot && toRelativePath(projectRoot, activePath) !== activePath
    const revealablePath = isUnderProjectRoot ? activePath : null

    const enabled = settings?.explorerAutoReveal ?? true
    const sidebarVisible = !(layout?.shellView?.zen ?? false) && !(layout?.shellView?.sidebarCollapsed ?? false)

    /**
     * The decision is taken here rather than during render because `lastRevealedPath` lives in a
     * ref — reading it while rendering would make the hook impure. Claiming the path before acting
     * is what makes the suppression hold even if the same decision is delivered twice (StrictMode's
     * double-mounted effect, a re-render arriving mid-reveal).
     */
    const applyAutoReveal = useEffectEvent((input: Omit<Parameters<typeof decideAutoReveal>[0], 'lastRevealedPath'>) => {
        const decision = decideAutoReveal({ ...input, lastRevealedPath: lastRevealedRef.current })
        if (decision === 'skip' || !input.activePath) return

        const path = input.activePath
        lastRevealedRef.current = path
        if (decision === 'select-only') {
            setSelectPathRequest(path)
            return
        }
        void (async () => {
            try {
                await revealTreeNode({ projectId, path })
            } catch {
                return
            }
            setSelectPathRequest(path)
        })()
    })

    useEffect(() => {
        const visiblePaths = new Set(rows.map((row) => row.path))
        applyAutoReveal({ enabled, activePath: revealablePath, visiblePaths, sidebarVisible, explorerViewActive })
    }, [enabled, revealablePath, rows, sidebarVisible, explorerViewActive])
}
