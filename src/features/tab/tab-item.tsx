import type { FC, MouseEvent, ReactNode } from 'react'
import { Pin, X } from 'lucide-react'
import { cn } from '@shared/lib/cn'

const MIDDLE_MOUSE_BUTTON = 1

type TabItemProps = {
    title: string
    icon: ReactNode
    active: boolean
    dirty: boolean
    pinned: boolean
    preview: boolean
    onActivate: () => void
    onClose: () => void
}

export const TabItem: FC<TabItemProps> = ({ title, icon, active, dirty, pinned, preview, onActivate, onClose }) => {
    const handleAuxClick = (event: MouseEvent) => {
        if (event.button !== MIDDLE_MOUSE_BUTTON) return
        event.preventDefault()
        onClose()
    }

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
            <span className='flex size-3.5 shrink-0 items-center justify-center'>{icon}</span>
            <span className={cn('truncate', preview && 'italic')}>{title}</span>
            <button
                type='button'
                aria-label={pinned ? `${title} 고정 해제` : `${title} 닫기`}
                onClick={(event) => {
                    event.stopPropagation()
                    onClose()
                }}
                className='ml-auto flex size-4 shrink-0 items-center justify-center rounded-sm hover:bg-white/10'>
                {dirty && <span className='bg-tab-bar-dirty-dot size-2 rounded-full group-hover:hidden' />}
                {pinned && !dirty && <Pin className='size-3' />}
                <X className={cn('size-3', (dirty || pinned) && 'hidden group-hover:block')} />
            </button>
        </div>
    )
}
