import { useQuery } from '@tanstack/react-query'
import { gitStatusQueryOptions } from '@entities/git/git.query'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { activeProjectQueryOptions, projectListQueryOptions } from '@entities/project/project.query'
import { findActiveTab } from '@shared/lib/pane-tree'
import { TitleBar } from '@features/window/title-bar'

export const TitleBarContent = () => {
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { data: layout } = useQuery(layoutQueryOptions(activeProjectId))
    const { data: gitStatus, isError: isGitError } = useQuery(gitStatusQueryOptions(activeProjectId))

    const project = projects.find((item) => item.id === activeProjectId) ?? null
    const activeTab = layout ? findActiveTab(layout.root, layout.focusedPane) : null

    return (
        <TitleBar tabTitle={activeTab?.title ?? null} projectName={project?.name ?? null} branch={isGitError ? null : (gitStatus?.branch ?? null)} />
    )
}
