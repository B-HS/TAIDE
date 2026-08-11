import type { FC, ReactNode } from 'react'
import { Switch } from '@shared/ui/switch'

type AgentHooksToggleProps = {
    label: ReactNode
    hint?: ReactNode
    checked: boolean
    disabled?: boolean
    onCheckedChange: (checked: boolean) => void
}

export const AgentHooksToggle: FC<AgentHooksToggleProps> = ({ label, hint, checked, disabled, onCheckedChange }) => (
    <label className='flex items-center justify-between gap-3 text-xs'>
        <span className='flex flex-col gap-0.5'>
            <span className='text-app-foreground'>{label}</span>
            {hint && <span className='text-app-sidebar-icon-default flex flex-col gap-0.5'>{hint}</span>}
        </span>
        <Switch checked={checked} disabled={disabled} onCheckedChange={onCheckedChange} />
    </label>
)
