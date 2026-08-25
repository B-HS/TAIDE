import type { FC } from 'react'
import { Switch } from '@shared/ui/switch'

type SwitchFieldProps = {
    label: string
    description?: string
    checked: boolean
    onCheckedChange: (checked: boolean) => void
}

export const SwitchField: FC<SwitchFieldProps> = ({ label, description, checked, onCheckedChange }) => (
    <label className='flex items-center justify-between gap-3 text-xs'>
        {description ? (
            <span className='flex flex-col gap-0.5'>
                <span className='text-app-foreground'>{label}</span>
                <span className='text-app-sidebar-icon-default'>{description}</span>
            </span>
        ) : (
            <span className='text-app-foreground'>{label}</span>
        )}
        <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </label>
)
