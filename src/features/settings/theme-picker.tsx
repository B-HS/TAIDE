import type { FC } from 'react'
import { Check, Copy } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { ThemeSummary } from '@shared/api/bindings'
import { BUILTIN_THEME_ID } from '@entities/theme/theme-tokens'
import { cn } from '@shared/lib/cn'
import { IconButton } from '@shared/ui/icon-button'

type ThemePickerProps = {
    themes: ThemeSummary[]
    activeThemeId: string
    onSelect: (themeId: string) => void
    onDuplicate?: (themeId: string) => void
}

const TAIDE_BUILTIN_IDS: readonly string[] = [BUILTIN_THEME_ID.DARK, BUILTIN_THEME_ID.LIGHT]

const isTaideBuiltin = (theme: ThemeSummary) => TAIDE_BUILTIN_IDS.includes(theme.id)

type ThemePickerSectionProps = {
    title: string
    themes: ThemeSummary[]
    activeThemeId: string
    onSelect: (themeId: string) => void
    onDuplicate?: (themeId: string) => void
}

const ThemePickerSection: FC<ThemePickerSectionProps> = ({ title, themes, activeThemeId, onSelect, onDuplicate }) => {
    const { t } = useTranslation()
    if (themes.length === 0) return null

    return (
        <div className='flex flex-col gap-2'>
            <div className='text-app-sidebar-icon-default text-xs font-medium'>{title}</div>
            <div className='grid grid-cols-2 gap-2 sm:grid-cols-3'>
                {themes.map((theme) => {
                    const isActive = theme.id === activeThemeId
                    return (
                        <div
                            key={theme.id}
                            className={cn(
                                'border-app-border hover:bg-app-sidebar-item-hover flex min-w-0 items-center justify-between gap-2 rounded-md border px-3 py-2 text-left text-xs',
                                isActive && 'border-app-focus-border bg-app-sidebar-item-active',
                            )}>
                            <button
                                type='button'
                                onClick={() => onSelect(theme.id)}
                                aria-pressed={isActive}
                                className='flex min-w-0 flex-1 flex-col text-left'>
                                <span className='text-app-foreground truncate font-medium'>{theme.name}</span>
                                <span className='text-app-sidebar-icon-default truncate'>
                                    {theme.type === 'dark' ? t('settings.themeDark') : t('settings.themeLight')}
                                </span>
                            </button>
                            <div className='flex shrink-0 items-center gap-1'>
                                {onDuplicate && (
                                    <IconButton
                                        onClick={() => onDuplicate(theme.id)}
                                        label={t('themeEditor.duplicateTheme')}
                                        icon={<Copy className='size-3.5' />}
                                        side='bottom'
                                        className='text-app-sidebar-icon-default hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'
                                    />
                                )}
                                {isActive && <Check className='text-app-accent size-4 shrink-0' />}
                            </div>
                        </div>
                    )
                })}
            </div>
        </div>
    )
}

export const ThemePicker: FC<ThemePickerProps> = ({ themes, activeThemeId, onSelect, onDuplicate }) => {
    const { t } = useTranslation()
    const builtinThemes = themes.filter((theme) => theme.builtin && isTaideBuiltin(theme))
    const bundledThemes = themes.filter((theme) => theme.builtin && !isTaideBuiltin(theme))
    const customThemes = themes.filter((theme) => !theme.builtin)

    return (
        <div className='flex flex-col gap-4'>
            <ThemePickerSection title={t('settings.builtinThemesSection')} themes={builtinThemes} activeThemeId={activeThemeId} onSelect={onSelect} />
            <ThemePickerSection
                title={t('settings.bundledThemesSection')}
                themes={bundledThemes}
                activeThemeId={activeThemeId}
                onSelect={onSelect}
                onDuplicate={onDuplicate}
            />
            <ThemePickerSection
                title={t('themeEditor.customThemes')}
                themes={customThemes}
                activeThemeId={activeThemeId}
                onSelect={onSelect}
                onDuplicate={onDuplicate}
            />
        </div>
    )
}
