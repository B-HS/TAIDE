import type { FC, ReactNode } from 'react'
import { cn } from '@shared/lib/cn'

type ProjectIconButtonProps = {
    name: string
    icon: ReactNode
    active: boolean
    agentRunning: boolean
    onActivate: () => void
}

const initialsOf = (name: string) => name.trim().slice(0, 2).toUpperCase()

export const ProjectIconButton: FC<ProjectIconButtonProps> = ({ name, icon, active, agentRunning, onActivate }) => (
    <button
        type='button'
        aria-label={name}
        aria-current={active}
        onClick={onActivate}
        className={cn(
            'relative flex size-10 shrink-0 items-center justify-center rounded-md text-xs font-medium',
            'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover',
            active && 'bg-app-sidebar-item-active text-app-foreground',
        )}>
        {active && <span className='bg-app-accent absolute top-1/2 left-0 h-5 w-0.5 -translate-x-1.5 -translate-y-1/2 rounded-full' />}
        {icon ?? initialsOf(name)}
        {agentRunning && <span className='bg-app-sidebar-icon-agent-running absolute right-1 bottom-1 size-2 rounded-full' />}
    </button>
)
