import type { FC } from 'react'
import { useState } from 'react'
import { CheckCircle2, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { REMOTE_PASSWORD_MIN_LEN } from '@entities/remote/remote.ipc'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'

type RemotePasswordRowProps = {
    configured: boolean
    warning?: string
    saving: boolean
    onSave: (password: string) => void
    onClear: () => void
}

export const RemotePasswordRow: FC<RemotePasswordRowProps> = ({ configured, warning, saving, onSave, onClear }) => {
    const { t } = useTranslation()
    const [password, setPassword] = useState('')

    const trimmed = password.trim()
    const isTooShort = trimmed.length > 0 && trimmed.length < REMOTE_PASSWORD_MIN_LEN

    const handleSave = () => {
        if (!trimmed || isTooShort) return
        setPassword('')
        onSave(trimmed)
    }

    return (
        <div className='border-app-border flex min-w-0 flex-col gap-1.5 rounded-md border px-3 py-2 text-xs'>
            <div className='flex min-w-0 items-center gap-2'>
                {configured ? (
                    <CheckCircle2 className='text-app-sidebar-icon-agent-running size-3.5 shrink-0' />
                ) : (
                    <XCircle className='text-app-sidebar-badge size-3.5 shrink-0' />
                )}
                <span className='text-app-foreground min-w-0 truncate font-medium'>{t('remote.passwordLabel')}</span>
                <span className={cn('ml-auto shrink-0', configured ? 'text-app-sidebar-icon-agent-running' : 'text-app-sidebar-badge')}>
                    {configured ? t('remote.passwordConfigured') : t('remote.passwordNotConfigured')}
                </span>
            </div>
            {warning && <span className='text-status-warning min-w-0 pl-5.5'>{warning}</span>}
            {isTooShort && (
                <span className='text-status-warning min-w-0 pl-5.5'>{t('remote.passwordTooShort', { min: REMOTE_PASSWORD_MIN_LEN })}</span>
            )}
            <div className='flex min-w-0 items-center gap-1.5 pl-5.5'>
                <input
                    type='password'
                    value={password}
                    placeholder={t('remote.passwordPlaceholder')}
                    onChange={(event) => setPassword(event.target.value)}
                    className='bg-panel-input-background border-panel-input-border text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1'
                />
                <Button type='button' variant='outline' size='xs' disabled={!trimmed || isTooShort || saving} onClick={handleSave}>
                    {t('remote.passwordSet')}
                </Button>
                {configured && (
                    <Button type='button' variant='ghost' size='xs' onClick={onClear}>
                        {t('remote.passwordClear')}
                    </Button>
                )}
            </div>
        </div>
    )
}
