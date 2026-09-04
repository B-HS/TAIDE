import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { AgentActivity } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import type { ProjectDisplayResolution } from '@shared/lib/project-display'
import { AgentStatusBadge } from '@features/project/agent-status-badge'
import { ProjectDisplayGlyph } from '@features/project/project-display-glyph'

type ProjectIconButtonProps = {
    name: string
    display: ProjectDisplayResolution
    active: boolean
    agentActivity: AgentActivity | null
    badgeEnabled: boolean
    onActivate: () => void
}

/**
 * `aria-label` stays the project's full name even when a 1–4 character label replaces the glyph: the
 * short label is a visual abbreviation, and letting it reach the accessibility tree would leave a
 * screen reader announcing `TA` for a project the tooltip calls by its real name.
 *
 * `overflow-hidden` is what makes the label promise ("no overflow") hold. The type ladder in
 * `resolveProjectLabelClassName` shrinks four full-width characters to fit the 40px button, but a
 * record written by an older build — or a font wider than the ladder assumes — must clip inside the
 * button rather than spill over the neighbouring project and its agent badge. It sits on an inner
 * wrapper rather than the button itself because the active indicator is drawn *outside* the button
 * box (`-translate-x-1.5`), and clipping at the button would erase it.
 */
export const ProjectIconButton: FC<ProjectIconButtonProps> = ({ name, display, active, agentActivity, badgeEnabled, onActivate }) => {
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
            <span className='flex max-w-full items-center justify-center overflow-hidden'>
                <ProjectDisplayGlyph display={display} />
            </span>
            {visibleActivity && <AgentStatusBadge activity={visibleActivity} />}
        </button>
    )
}
