import type { FC } from 'react'
import { CheckCircle2, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { RemoteStatus } from '@shared/api/bindings'
import { RemoteAllowedHostsRow } from '@features/settings/remote-allowed-hosts-row'
import { RemotePasswordRow } from '@features/settings/remote-password-row'
import { Button } from '@shared/ui/button'
import { Switch } from '@shared/ui/switch'

type RemoteSectionProps = {
    status: RemoteStatus | undefined
    enabled: boolean
    issuedUrl: string | null
    issuing: boolean
    revoking: boolean
    passwordSaving: boolean
    passwordOnlyLogin: boolean
    allowedHosts: string[]
    allowedHostsSaving: boolean
    onToggle: (enabled: boolean) => void
    onIssueLink: () => void
    onRevokeSessions: () => void
    onSavePassword: (password: string) => void
    onClearPassword: () => void
    onTogglePasswordOnlyLogin: (enabled: boolean) => void
    onChangeAllowedHosts: (hosts: string[]) => void
}

export const RemoteSection: FC<RemoteSectionProps> = ({
    status,
    enabled,
    issuedUrl,
    issuing,
    revoking,
    passwordSaving,
    passwordOnlyLogin,
    allowedHosts,
    allowedHostsSaving,
    onToggle,
    onIssueLink,
    onRevokeSessions,
    onSavePassword,
    onClearPassword,
    onTogglePasswordOnlyLogin,
    onChangeAllowedHosts,
}) => {
    const { t } = useTranslation()
    const running = status?.running ?? false
    const passwordConfigured = status?.passwordConfigured ?? false

    return (
        <div className='flex flex-col gap-2 text-xs'>
            <div className='flex items-center justify-between gap-3'>
                <div className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('remote.enableToggle')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('remote.enableToggleHint')}</span>
                </div>
                <Switch checked={enabled} onCheckedChange={onToggle} />
            </div>
            <div className='flex items-center gap-2'>
                {running ? (
                    <CheckCircle2 className='text-app-sidebar-icon-agent-running size-3.5 shrink-0' />
                ) : (
                    <XCircle className='text-app-sidebar-icon-default size-3.5 shrink-0' />
                )}
                <span className='text-app-foreground'>
                    {running ? t('remote.statusRunning', { port: status?.port ?? 0 }) : t('remote.statusStopped')}
                </span>
                {running && (
                    <span className='text-app-sidebar-icon-default ml-auto'>{t('remote.clientCountLabel', { count: status?.clientCount ?? 0 })}</span>
                )}
            </div>
            <p className='text-status-warning'>{t('remote.securityWarning')}</p>
            <RemotePasswordRow
                configured={passwordConfigured}
                warning={passwordConfigured ? undefined : t('remote.passwordHint')}
                saving={passwordSaving}
                onSave={onSavePassword}
                onClear={onClearPassword}
            />
            {passwordConfigured && (
                <label className='flex items-center justify-between gap-3'>
                    <div className='flex flex-col gap-0.5'>
                        <span className='text-app-foreground'>{t('remote.passwordOnlyToggle')}</span>
                        <span className='text-app-sidebar-icon-default'>{t('remote.passwordOnlyToggleDescription')}</span>
                    </div>
                    <Switch checked={passwordOnlyLogin} onCheckedChange={onTogglePasswordOnlyLogin} />
                </label>
            )}
            <RemoteAllowedHostsRow hosts={allowedHosts} saving={allowedHostsSaving} onChange={onChangeAllowedHosts} />
            {issuedUrl && <span className='text-app-foreground break-all select-all'>{issuedUrl}</span>}
            <div className='flex items-center gap-1.5'>
                <Button type='button' variant='outline' size='xs' disabled={!running || issuing} onClick={onIssueLink}>
                    {t('remote.issueLink')}
                </Button>
                <Button type='button' variant='ghost' size='xs' className='ml-auto' disabled={!running || revoking} onClick={onRevokeSessions}>
                    <XCircle className='size-3.5' />
                    {t('remote.revokeSessions')}
                </Button>
            </div>
        </div>
    )
}
