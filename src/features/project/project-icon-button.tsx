import type { FC, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { AgentActivity } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { AgentStatusBadge } from '@features/project/agent-status-badge'

type ProjectIconButtonProps = {
    name: string
    icon: ReactNode
    active: boolean
    agentActivity: AgentActivity | null
    badgeEnabled: boolean
    onActivate: () => void
}

const initialsOf = (name: string) => name.trim().slice(0, 2).toUpperCase()

export const ProjectIconButton: FC<ProjectIconButtonProps> = ({ name, icon, active, agentActivity, badgeEnabled, onActivate }) => {
    const { t } = useTranslation()
    const visibleActivity = badgeEnabled ? agentActivity : null

    return (
        <button
            type='button'
            aria-label={visibleActivity ? `${name} — ${t(`agent.status.${visibleActivity}`)}` : name}
            aria-current={active}
            onClick={onActivate}
            className={cn(
                'relative flex size-10 shrink-0 items-center justify-center rounded-md text-xs font-medium',
                'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover',
                active && 'bg-app-sidebar-item-active text-app-foreground',
            )}>
            {active && <span className='bg-app-accent absolute top-1/2 left-0 h-5 w-0.5 -translate-x-1.5 -translate-y-1/2 rounded-full' />}
            {icon ?? initialsOf(name)}
            {visibleActivity && <AgentStatusBadge activity={visibleActivity} />}
        </button>
    )
}
