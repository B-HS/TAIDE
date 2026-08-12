import type { FC } from 'react'
import { Copy, Pencil } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { ThemeSummary } from '@shared/api/bindings'
import { IconButton } from '@shared/ui/icon-button'

type CustomThemeListProps = {
    themes: ThemeSummary[]
    onEdit: (themeId: string) => void
    onDuplicate: (themeId: string) => void
}

export const CustomThemeList: FC<CustomThemeListProps> = ({ themes, onEdit, onDuplicate }) => {
    const { t } = useTranslation()

    if (themes.length === 0) return <span className='text-app-sidebar-icon-default text-xs'>{t('themeEditor.noCustomThemes')}</span>

    return (
        <div className='flex flex-col gap-1'>
            {themes.map((theme) => (
                <div key={theme.id} className='border-app-border flex items-center justify-between gap-2 rounded-sm border px-2 py-1.5'>
                    <span className='text-app-foreground truncate text-xs'>{theme.name}</span>
                    <div className='flex shrink-0 items-center gap-1'>
                        <IconButton
                            onClick={() => onDuplicate(theme.id)}
                            label={t('themeEditor.duplicateTheme')}
                            icon={<Copy className='size-3.5' />}
                            side='bottom'
                            className='text-app-sidebar-icon-default hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'
                        />
                        <IconButton
                            onClick={() => onEdit(theme.id)}
                            label={t('themeEditor.editTheme')}
                            icon={<Pencil className='size-3.5' />}
                            side='bottom'
                            className='text-app-sidebar-icon-default hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'
                        />
                    </div>
                </div>
            ))}
        </div>
    )
}
