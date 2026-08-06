import type { FC } from 'react'
import { Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { ThemeSummary } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'

type ThemePickerProps = {
    themes: ThemeSummary[]
    activeThemeId: string
    onSelect: (themeId: string) => void
}

export const ThemePicker: FC<ThemePickerProps> = ({ themes, activeThemeId, onSelect }) => {
    const { t } = useTranslation()

    return (
        <div className='grid grid-cols-2 gap-2 sm:grid-cols-3'>
            {themes.map((theme) => {
                const isActive = theme.id === activeThemeId
                return (
                    <button
                        key={theme.id}
                        type='button'
                        onClick={() => onSelect(theme.id)}
                        aria-pressed={isActive}
                        className={cn(
                            'border-app-border hover:bg-app-sidebar-item-hover flex items-center justify-between rounded-md border px-3 py-2 text-left text-xs',
                            isActive && 'border-app-focus-border bg-app-sidebar-item-active',
                        )}>
                        <span className='flex flex-col'>
                            <span className='text-app-foreground font-medium'>{theme.name}</span>
                            <span className='text-app-sidebar-icon-default'>
                                {theme.type === 'dark' ? t('settings.themeDark') : t('settings.themeLight')}
                            </span>
                        </span>
                        {isActive && <Check className='text-app-accent size-4 shrink-0' />}
                    </button>
                )
            })}
        </div>
    )
}
