import type { FC } from 'react'

type GitSectionCountBadgeProps = {
    count: number
}

/**
 * The pill that carries a git section's item count.
 *
 * Its own component because two headers draw it and only one of them is a `GitSectionHeader`:
 * the commit detail panel's "Changed Files" heading is a plain caption — no chevron, no toggle, no
 * sticky positioning, and no roving-focus stop — so it cannot reuse the section header itself, and
 * inlining the pill there instead left the two copies free to drift apart on the next restyle.
 */
export const GitSectionCountBadge: FC<GitSectionCountBadgeProps> = ({ count }) => (
    <span className='bg-app-sidebar-item-active flex h-4 min-w-4 shrink-0 items-center justify-center rounded-full px-1 text-[10px] font-normal normal-case'>
        {count}
    </span>
)
