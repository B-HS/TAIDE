import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { Switch } from '@shared/ui/switch'

type AgentHooksToggleProps = {
    checked: boolean
    disabled?: boolean
    onCheckedChange: (checked: boolean) => void
}

export const AgentHooksToggle: FC<AgentHooksToggleProps> = ({ checked, disabled, onCheckedChange }) => {
    const { t } = useTranslation()

    return (
        <label className='flex items-center justify-between gap-3 text-xs'>
            <span className='flex flex-col gap-0.5'>
                <span className='text-app-foreground'>{t('settings.agentHooks')}</span>
                <span className='text-app-sidebar-icon-default'>{t('settings.agentHooksHint')}</span>
            </span>
            <Switch checked={checked} disabled={disabled} onCheckedChange={onCheckedChange} />
        </label>
    )
}
