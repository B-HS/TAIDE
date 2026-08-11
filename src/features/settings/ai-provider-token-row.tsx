import type { FC } from 'react'
import { useState } from 'react'
import { CheckCircle2, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'

type AiProviderTokenRowProps = {
    label: string
    warning?: string
    configured: boolean
    saving: boolean
    onSave: (token: string) => void
    onClear: () => void
}

export const AiProviderTokenRow: FC<AiProviderTokenRowProps> = ({ label, warning, configured, saving, onSave, onClear }) => {
    const { t } = useTranslation()
    const [token, setToken] = useState('')

    const handleSave = () => {
        const trimmed = token.trim()
        if (!trimmed) return
        setToken('')
        onSave(trimmed)
    }

    return (
        <li className='border-app-border flex min-w-0 flex-col gap-1.5 rounded-md border px-3 py-2 text-xs'>
            <div className='flex min-w-0 items-center gap-2'>
                {configured ? (
                    <CheckCircle2 className='text-app-sidebar-icon-agent-running size-3.5 shrink-0' />
                ) : (
                    <XCircle className='text-app-sidebar-badge size-3.5 shrink-0' />
                )}
                <span className='text-app-foreground min-w-0 truncate font-medium'>{label}</span>
                <span className={cn('ml-auto shrink-0', configured ? 'text-app-sidebar-icon-agent-running' : 'text-app-sidebar-badge')}>
                    {configured ? t('settings.aiTokenSaved') : t('settings.aiTokenNotSet')}
                </span>
            </div>
            {warning && <span className='text-status-warning min-w-0 pl-5.5'>{warning}</span>}
            <div className='flex min-w-0 items-center gap-1.5 pl-5.5'>
                <input
                    type='password'
                    value={token}
                    placeholder={t('settings.aiTokenPlaceholder')}
                    onChange={(event) => setToken(event.target.value)}
                    className='bg-panel-input-background border-panel-input-border text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1'
                />
                <Button type='button' variant='outline' size='xs' disabled={!token.trim() || saving} onClick={handleSave}>
                    {t('settings.aiTokenSave')}
                </Button>
                {configured && (
                    <Button type='button' variant='ghost' size='xs' onClick={onClear}>
                        {t('settings.aiTokenClear')}
                    </Button>
                )}
            </div>
        </li>
    )
}
