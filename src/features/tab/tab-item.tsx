import type { FC, MouseEvent, ReactNode } from 'react'
import { Pin, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { ICON_BUTTON_CLASS } from '@shared/constants/ui-class'
import { IconButton } from '@shared/ui/icon-button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const MIDDLE_MOUSE_BUTTON = 1

type TabItemProps = {
    title: string
    icon: ReactNode
    active: boolean
    dirty: boolean
    pinned: boolean
    preview: boolean
    agentTooltip?: string
    onActivate: () => void
    onClose: () => void
    onTogglePin: () => void
}

/**
 * A pinned tab is protected from every closing gesture this component owns (`docs/features/tabs.md`
 * §3): the trailing icon button unpins instead of closing — its label already said "unpin" while it
 * ran `onClose`, so one click on what looked like the unpin affordance discarded the tab — and a
 * middle click is ignored outright. ⌘W's matching guard lives in `editor-area.tsx`, which owns that
 * keymap handler.
 */
export const TabItem: FC<TabItemProps> = ({ title, icon, active, dirty, pinned, preview, agentTooltip, onActivate, onClose, onTogglePin }) => {
    const { t } = useTranslation()

    const handleAuxClick = (event: MouseEvent) => {
        if (event.button !== MIDDLE_MOUSE_BUTTON) return
        event.preventDefault()
        if (pinned) return
        onClose()
    }

    const iconSlot = <span className='flex size-3.5 shrink-0 items-center justify-center'>{icon}</span>

    return (
        <div
            role='tab'
            aria-selected={active}
            tabIndex={active ? 0 : -1}
            onClick={onActivate}
            onAuxClick={handleAuxClick}
            className={cn(
                'group relative flex h-9 max-w-52 min-w-24 shrink-0 cursor-pointer items-center gap-1.5 border-r px-3 text-xs select-none',
                'border-tab-bar-tab-border',
                active
                    ? 'bg-tab-bar-tab-active-background text-tab-bar-tab-active-foreground'
                    : 'bg-tab-bar-tab-inactive-background text-tab-bar-tab-inactive-foreground hover:text-tab-bar-tab-active-foreground',
            )}>
            {active && <span className='bg-tab-bar-tab-active-indicator absolute inset-x-0 top-0 h-0.5' />}
            {agentTooltip ? (
                <Tooltip>
                    <TooltipTrigger asChild>{iconSlot}</TooltipTrigger>
                    <TooltipContent side='bottom'>{agentTooltip}</TooltipContent>
                </Tooltip>
            ) : (
                iconSlot
            )}
            <span className={cn('truncate', preview && 'text-tab-bar-preview-foreground italic')}>{title}</span>
            <IconButton
                label={pinned ? t('tab.unpinAriaLabel', { title }) : t('tab.closeAriaLabel', { title })}
                icon={
                    <>
                        {dirty && <span className='bg-tab-bar-dirty-dot size-2 rounded-full group-hover:hidden' />}
                        {pinned ? (
                            <Pin className={cn('size-3', dirty && 'hidden group-hover:block')} />
                        ) : (
                            <X className={cn('size-3', dirty && 'hidden group-hover:block')} />
                        )}
                    </>
                }
                onClick={(event) => {
                    event.stopPropagation()
                    if (pinned) return onTogglePin()
                    onClose()
                }}
                side='bottom'
                className={cn(ICON_BUTTON_CLASS, 'ml-auto size-4 shrink-0')}
            />
        </div>
    )
}
