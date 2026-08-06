import type { FC } from 'react'
import { Check } from 'lucide-react'
import type { ShellProfile } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'

type ShellProfileListProps = {
    profiles: ShellProfile[]
    activePath: string | null
    onSelect: (path: string) => void
}

export const ShellProfileList: FC<ShellProfileListProps> = ({ profiles, activePath, onSelect }) => (
    <ul className='flex flex-col gap-1'>
        {profiles.map((profile) => {
            const isActive = profile.path === activePath
            return (
                <li key={profile.id}>
                    <button
                        type='button'
                        onClick={() => onSelect(profile.path)}
                        aria-pressed={isActive}
                        className={cn(
                            'border-app-border hover:bg-app-sidebar-item-hover flex w-full items-center justify-between rounded-sm border px-3 py-1.5 text-left text-xs',
                            isActive && 'border-app-focus-border bg-app-sidebar-item-active',
                        )}>
                        <span className='flex flex-col'>
                            <span className='text-app-foreground font-medium'>{profile.name}</span>
                            <span className='text-app-sidebar-icon-default font-mono'>{profile.path}</span>
                        </span>
                        {isActive && <Check className='text-app-accent size-4 shrink-0' />}
                    </button>
                </li>
            )
        })}
    </ul>
)
