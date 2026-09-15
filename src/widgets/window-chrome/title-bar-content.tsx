import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { gitStatusQueryOptions } from '@entities/git/git.query'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { projectListQueryOptions } from '@entities/project/project.query'
import { findActiveTab } from '@shared/lib/pane-tree'
import { TitleBar } from '@features/window/title-bar'

type TitleBarContentProps = {
    projectId: ProjectId | null
}

/**
 * Scoped to the project its host hands it rather than reading the global active-project session
 * itself (d-62 §1.B): the main window's title bar follows whichever shell slot has focus, which only
 * `AppShell` — the owner of the slot tree — can answer.
 */
export const TitleBarContent: FC<TitleBarContentProps> = ({ projectId }) => {
    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { data: gitStatus, isError: isGitError } = useQuery(gitStatusQueryOptions(projectId))

    const project = projects.find((item) => item.id === projectId) ?? null
    const activeTab = layout ? findActiveTab(layout.root, layout.focusedPane) : null

    return (
        <TitleBar tabTitle={activeTab?.title ?? null} projectName={project?.name ?? null} branch={isGitError ? null : (gitStatus?.branch ?? null)} />
    )
}
