import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { HookInstallScope, Project } from '@shared/api/bindings'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { AgentHooksProjectRow } from '@widgets/settings-view/agent-hooks-project-row'
import { AgentHooksToggle } from '@features/settings/agent-hooks-toggle'
import {
    agentHooksStatusQueryOptions,
    cliInstallStatusQueryOptions,
    USER_LEVEL_AGENT_HOOKS_UNUSED_PROJECT_ID,
    useInstallAgentHooks,
    useUninstallAgentHooks,
} from '@entities/agent/agent.query'

type AgentHooksAgentOption = { name: string; labelKey: string; scope: HookInstallScope }

const AGENT_HOOKS_AGENTS: AgentHooksAgentOption[] = [
    { name: 'claude', labelKey: 'settings.agentHooksAgentClaude', scope: 'project' },
    { name: 'codex', labelKey: 'settings.agentHooksAgentCodex', scope: 'user' },
    { name: 'gemini', labelKey: 'settings.agentHooksAgentGemini', scope: 'user' },
]

type AgentHooksClaudeSectionProps = {
    labelKey: string
    projects: Project[]
}

const AgentHooksClaudeSection: FC<AgentHooksClaudeSectionProps> = ({ labelKey, projects }) => {
    const { t } = useTranslation()

    return (
        <div className='flex min-w-0 flex-col gap-1.5'>
            <span className='text-app-sidebar-icon-default text-xs'>{t(labelKey)}</span>
            {projects.length === 0 ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('app.noProjectOpen')}</span>
            ) : (
                <ul className='flex min-w-0 flex-col gap-1.5'>
                    {projects.map((project) => (
                        <AgentHooksProjectRow key={project.id} projectId={project.id} projectName={project.name} agentName='claude' />
                    ))}
                </ul>
            )}
        </div>
    )
}

type AgentHooksUserLevelRowProps = {
    agentName: string
    labelKey: string
}

const AgentHooksUserLevelRow: FC<AgentHooksUserLevelRowProps> = ({ agentName, labelKey }) => {
    const { t } = useTranslation()

    const { data: status, isPending } = useQuery(agentHooksStatusQueryOptions(USER_LEVEL_AGENT_HOOKS_UNUSED_PROJECT_ID, agentName))
    const { data: cliStatus, isPending: isCliPending } = useQuery(cliInstallStatusQueryOptions())
    const { mutate: installHooks, isPending: isInstalling } = useInstallAgentHooks()
    const { mutate: uninstallHooks, isPending: isUninstalling } = useUninstallAgentHooks()

    const isCliMissing = !isCliPending && !cliStatus?.installed

    const handleCheckedChange = (checked: boolean) => {
        const variables = { projectId: USER_LEVEL_AGENT_HOOKS_UNUSED_PROJECT_ID, agentName }
        const onError = (error: Error) => toast.error(describeIpcError(error))
        if (checked) installHooks(variables, { onError })
        else uninstallHooks(variables, { onError })
    }

    return (
        <AgentHooksToggle
            label={t(labelKey)}
            hint={
                <>
                    <span>{t('settings.agentHooksUserLevelDescription')}</span>
                    <span className='text-status-warning'>{t('settings.agentHooksUserLevelWarning')}</span>
                    {isCliMissing && <span className='text-status-warning'>{t('settings.agentHooksCliMissing')}</span>}
                </>
            }
            checked={status?.installed ?? false}
            disabled={isPending || isCliPending || isInstalling || isUninstalling || (isCliMissing && !status?.installed)}
            onCheckedChange={handleCheckedChange}
        />
    )
}

type AgentHooksProjectListProps = {
    projects: Project[]
}

export const AgentHooksProjectList: FC<AgentHooksProjectListProps> = ({ projects }) => (
    <div className='flex min-w-0 flex-col gap-3'>
        {AGENT_HOOKS_AGENTS.map((agent) =>
            agent.scope === 'project' ? (
                <AgentHooksClaudeSection key={agent.name} labelKey={agent.labelKey} projects={projects} />
            ) : (
                <AgentHooksUserLevelRow key={agent.name} agentName={agent.name} labelKey={agent.labelKey} />
            ),
        )}
    </div>
)
