import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { Switch } from '@shared/ui/switch'

type AiAutoTabToggleProps = {
    checked: boolean
    disabled: boolean
    onCheckedChange: (checked: boolean) => void
}

export const AiAutoTabToggle: FC<AiAutoTabToggleProps> = ({ checked, disabled, onCheckedChange }) => {
    const { t } = useTranslation()

    return (
        <label className='flex items-center justify-between gap-3 text-xs'>
            <span className='flex flex-col gap-0.5'>
                <span className='text-app-foreground'>{t('settings.aiAutoTabToggle')}</span>
                <span className='text-app-sidebar-icon-default'>
                    {disabled ? t('settings.aiAutoTabProviderRequired') : t('settings.aiAutoTabToggleHint')}
                </span>
            </span>
            <Switch checked={checked} disabled={disabled} onCheckedChange={onCheckedChange} />
        </label>
    )
}
