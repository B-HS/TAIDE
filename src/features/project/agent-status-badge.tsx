import type { FC } from 'react'
import type { AgentActivity } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'

type AgentStatusBadgeProps = {
    activity: AgentActivity
}

const BADGE_CLASS_NAME: Record<AgentActivity, string> = {
    working: 'size-2.5 rounded-full bg-app-sidebar-icon-agent-working motion-safe:animate-pulse',
    awaitingInput: 'size-3 rotate-45 bg-app-sidebar-icon-agent-awaiting',
    idle: 'size-2 rounded-full border border-app-sidebar-icon-agent-idle bg-transparent',
    unknown: 'size-2 rounded-full border border-app-sidebar-icon-agent-unknown bg-transparent',
}

export const AgentStatusBadge: FC<AgentStatusBadgeProps> = ({ activity }) => (
    <span aria-hidden className={cn('ring-app-sidebar-background absolute right-0 bottom-0 ring-2', BADGE_CLASS_NAME[activity])} />
)
