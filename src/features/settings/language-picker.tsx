import type { FC } from 'react'
import { Check } from 'lucide-react'
import type { LocaleSummary } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'

export const SYSTEM_LANGUAGE_ID = 'system'

type LanguagePickerProps = {
    locales: LocaleSummary[]
    activeLanguage: string
    systemLabel: string
    onSelect: (language: string) => void
}

export const LanguagePicker: FC<LanguagePickerProps> = ({ locales, activeLanguage, systemLabel, onSelect }) => {
    const options = [{ id: SYSTEM_LANGUAGE_ID, name: systemLabel, builtin: true }, ...locales]

    return (
        <div className='grid grid-cols-2 gap-2 sm:grid-cols-4'>
            {options.map((option) => {
                const isActive = option.id === activeLanguage
                return (
                    <button
                        key={option.id}
                        type='button'
                        onClick={() => onSelect(option.id)}
                        aria-pressed={isActive}
                        className={cn(
                            'border-app-border hover:bg-app-sidebar-item-hover flex items-center justify-between rounded-md border px-3 py-2 text-left text-xs',
                            isActive && 'border-app-focus-border bg-app-sidebar-item-active',
                        )}>
                        <span className='text-app-foreground font-medium'>{option.name}</span>
                        {isActive && <Check className='text-app-accent size-4 shrink-0' />}
                    </button>
                )
            })}
        </div>
    )
}
