import type { FC } from 'react'
import { cn } from '@shared/lib/cn'

type SettingsTocItem = {
    id: string
    label: string
}

type SettingsTocProps = {
    items: SettingsTocItem[]
    activeId: string
    onSelect: (id: string) => void
}

export const SettingsToc: FC<SettingsTocProps> = ({ items, activeId, onSelect }) => (
    <nav className='flex w-48 shrink-0 flex-col gap-0.5'>
        {items.map((item) => {
            const isActive = item.id === activeId
            return (
                <button
                    key={item.id}
                    type='button'
                    onClick={() => onSelect(item.id)}
                    aria-current={isActive}
                    className={cn(
                        'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground rounded-md px-3 py-1.5 text-left text-xs font-medium transition-colors',
                        isActive && 'bg-app-sidebar-item-active text-app-foreground',
                    )}>
                    {item.label}
                </button>
            )
        })}
    </nav>
)
