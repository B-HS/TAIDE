import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { gitStatusQueryOptions } from '@entities/git/git.query'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { findActiveTab, resolveWindowPaneTree } from '@shared/lib/pane-tree'
import { TitleBar } from '@features/window/title-bar'

type AuxiliaryTitleBarContentProps = {
    projectId: ProjectId
    windowSlot: number
}

/**
 * Auxiliary-window counterpart to `widgets/window-chrome/title-bar-content.tsx` — same `TitleBar`
 * presentational component, but every value is derived from this window's own fixed
 * `(projectId, windowSlot)` instead of the global active-project session, matching contract §3.1's
 * "ProjectActivated 무시(고정 뷰)".
 */
export const AuxiliaryTitleBarContent: FC<AuxiliaryTitleBarContentProps> = ({ projectId, windowSlot }) => {
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { data: gitStatus, isError: isGitError } = useQuery(gitStatusQueryOptions(projectId))

    const paneTree = layout ? resolveWindowPaneTree(layout, { kind: 'auxiliary', projectId, windowSlot }) : null
    const activeTab = paneTree ? findActiveTab(paneTree.root, paneTree.focusedPane) : null

    return (
        <TitleBar tabTitle={activeTab?.title ?? null} projectName={project?.name ?? null} branch={isGitError ? null : (gitStatus?.branch ?? null)} />
    )
}
