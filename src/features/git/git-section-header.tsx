import type { FC, KeyboardEvent, MouseEvent, ReactNode } from 'react'
import { ChevronRight } from 'lucide-react'
import { cn } from '@shared/lib/cn'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'
import { GitSectionCountBadge } from '@features/git/git-section-count-badge'
import { IconButton } from '@shared/ui/icon-button'

export type GitSectionHeaderAction = {
    id: string
    label: string
    icon: ReactNode
    onClick: () => void
}

/**
 * Left inset for the rows a {@link GitSectionHeader} owns. It is the only thing that separates a
 * row from a header at a glance once the headers stopped carrying a background tone of their own,
 * and it lines the row text up under the header title (the header spends the same distance on its
 * chevron).
 */
export const GIT_SECTION_ROW_INDENT_CLASS = 'pl-4'

type GitSectionHeaderProps = {
    title: string
    count: number
    expanded: boolean
    onToggle: () => void
    actions?: GitSectionHeaderAction[]
}

/**
 * The one header every SCM panel section uses — resource groups, stashes and the commit graph — so
 * the panel reads as a stack of peers instead of the two competing header styles it had (the graph
 * carried its own inline markup).
 *
 * Three details are load-bearing:
 *
 * - `sticky top-0` with an explicit `bg-explorer-background`. Sticky is what tells the user which
 *   group the row under the cursor belongs to in a long list; the background has to be spelled out
 *   because a transparent sticky header lets the scrolled rows show through it.
 * - Separation is a `border-t` plus the row indent the caller applies, *not* a static background
 *   tone. Imported vsix themes routinely map the list-hover and list-background colors close
 *   together (`docs/theme-system.md` §8.2.2·§8.2.4), so a tinted header would collapse into the row
 *   hover feedback in exactly those themes.
 * - The count badge renders whether or not the section is expanded. A collapsed "Staged Changes"
 *   that hid its count would read as "nothing is staged", and acting on that belief routes the
 *   commit through the stage-everything confirmation instead.
 *
 * `data-git-section-header` marks it as a stop in the panel's roving focus order
 * (`change-row-navigation.ts`), which is also what makes the group actions reachable from the
 * keyboard: `group-focus-within:flex` reveals them once the header itself holds focus.
 */
export const GitSectionHeader: FC<GitSectionHeaderProps> = ({ title, count, expanded, onToggle, actions = [] }) => {
    const activate = createActivationKeyDownHandler(onToggle)

    /**
     * ArrowRight expands and ArrowLeft collapses, matching the file tree and VS Code's SCM groups —
     * pressing the key that would be a no-op falls through to the plain activation handler instead.
     * ArrowUp/ArrowDown are deliberately left alone: the panel container owns moving focus between
     * headers and rows.
     */
    const isArrowToggle = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.target !== event.currentTarget) return false
        if (event.key === 'ArrowRight') return !expanded
        return event.key === 'ArrowLeft' && expanded
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (!isArrowToggle(event)) return activate(event)
        event.preventDefault()
        onToggle()
    }

    /** Group actions live inside the header's own click target, so they must not toggle it too. */
    const handleActionClick = (action: GitSectionHeaderAction) => (event: MouseEvent<HTMLButtonElement>) => {
        event.stopPropagation()
        action.onClick()
    }

    return (
        <div
            role='button'
            tabIndex={0}
            aria-expanded={expanded}
            data-git-section-header
            onClick={onToggle}
            onKeyDown={handleKeyDown}
            className='group text-panel-section-header bg-explorer-background border-app-border hover:bg-explorer-item-hover focus-within:bg-explorer-item-focused sticky top-0 z-10 flex h-6 cursor-default items-center gap-1.5 border-t px-2 text-[11px] font-semibold tracking-wide uppercase outline-none select-none'>
            <ChevronRight className={cn('size-3 shrink-0', expanded && 'rotate-90')} />
            <span className='truncate'>{title}</span>
            <GitSectionCountBadge count={count} />
            {actions.length > 0 && (
                <span className='ml-auto hidden shrink-0 items-center gap-0.5 group-hover:flex group-focus-within:flex'>
                    {actions.map((action) => (
                        <IconButton
                            key={action.id}
                            label={action.label}
                            icon={action.icon}
                            onClick={handleActionClick(action)}
                            side='bottom'
                            className='hover:bg-explorer-item-selected flex size-4 items-center justify-center rounded-sm'
                        />
                    ))}
                </span>
            )}
        </div>
    )
}
