import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { Project } from '@shared/api/bindings'
import { AgentHooksProjectRow } from '@features/settings/agent-hooks-project-row'

type AgentHooksProjectListProps = {
    projects: Project[]
}

export const AgentHooksProjectList: FC<AgentHooksProjectListProps> = ({ projects }) => {
    const { t } = useTranslation()

    if (projects.length === 0) return <span className='text-app-sidebar-icon-default text-xs'>{t('app.noProjectOpen')}</span>

    return (
        <ul className='flex min-w-0 flex-col gap-1.5'>
            {projects.map((project) => (
                <AgentHooksProjectRow key={project.id} projectId={project.id} projectName={project.name} />
            ))}
        </ul>
    )
}
