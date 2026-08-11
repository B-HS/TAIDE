import type { FC } from 'react'
import { useState } from 'react'
import { CheckCircle2, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { SyncStatus } from '@shared/api/bindings'
import { Button } from '@shared/ui/button'

const GITHUB_TOKEN_CREATE_URL = 'https://github.com/settings/tokens/new?scopes=gist&description=TAIDE%20Sync'

type SyncSectionProps = {
    status: SyncStatus | undefined
    gistId: string | null
    connecting: boolean
    disconnecting: boolean
    uploading: boolean
    downloading: boolean
    onConnect: (pat: string) => void
    onDisconnect: () => void
    onUpload: () => void
    onDownload: () => void
}

export const SyncSection: FC<SyncSectionProps> = ({
    status,
    gistId,
    connecting,
    disconnecting,
    uploading,
    downloading,
    onConnect,
    onDisconnect,
    onUpload,
    onDownload,
}) => {
    const { t } = useTranslation()
    const [token, setToken] = useState('')

    const handleConnect = () => {
        const trimmed = token.trim()
        if (!trimmed) return
        setToken('')
        onConnect(trimmed)
    }

    if (!status?.connected)
        return (
            <div className='flex flex-col gap-2 text-xs'>
                <p className='text-app-sidebar-icon-default'>{t('settings.syncDescription')}</p>
                <p className='text-app-sidebar-icon-default flex flex-col gap-0.5'>
                    <span>{t('settings.syncTokenScopeHint')}</span>
                    <span className='text-app-foreground select-all'>{GITHUB_TOKEN_CREATE_URL}</span>
                </p>
                <div className='flex items-center gap-1.5'>
                    <input
                        type='password'
                        value={token}
                        placeholder={t('settings.syncTokenPlaceholder')}
                        onChange={(event) => setToken(event.target.value)}
                        className='bg-panel-input-background border-panel-input-border text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1'
                    />
                    <Button type='button' variant='outline' size='xs' disabled={!token.trim() || connecting} onClick={handleConnect}>
                        {t('settings.syncConnect')}
                    </Button>
                </div>
            </div>
        )

    return (
        <div className='flex flex-col gap-2 text-xs'>
            <div className='flex items-center gap-2'>
                <CheckCircle2 className='text-app-sidebar-icon-agent-running size-3.5 shrink-0' />
                <span className='text-app-foreground font-medium'>{t('settings.syncSectionTitle')}</span>
                {status.remoteNewer && (
                    <span className='bg-status-warning/15 text-status-warning ml-auto shrink-0 rounded-sm px-1.5 py-0.5'>
                        {t('settings.syncRemoteNewerBadge')}
                    </span>
                )}
            </div>
            <div className='flex items-center justify-between gap-3 pl-5.5'>
                <span className='text-app-sidebar-icon-default'>{t('settings.syncGistIdLabel')}</span>
                <span className='text-app-foreground min-w-0 truncate'>{gistId ?? t('settings.syncGistIdNone')}</span>
            </div>
            <div className='flex items-center justify-between gap-3 pl-5.5'>
                <span className='text-app-sidebar-icon-default'>{t('settings.syncLastSyncedLabel')}</span>
                <span className='text-app-foreground min-w-0 truncate'>
                    {status.lastSyncedAt ? new Date(status.lastSyncedAt).toLocaleString() : t('settings.syncLastSyncedNever')}
                </span>
            </div>
            <p className='text-status-warning pl-5.5'>{t('settings.syncSecretGistWarning')}</p>
            <div className='flex items-center gap-1.5 pl-5.5'>
                <Button type='button' variant='outline' size='xs' disabled={uploading} onClick={onUpload}>
                    {t('settings.syncUploadNow')}
                </Button>
                <Button type='button' variant='outline' size='xs' disabled={downloading} onClick={onDownload}>
                    {t('settings.syncDownloadNow')}
                </Button>
                <Button type='button' variant='ghost' size='xs' className='ml-auto' disabled={disconnecting} onClick={onDisconnect}>
                    <XCircle className='size-3.5' />
                    {t('settings.syncDisconnect')}
                </Button>
            </div>
        </div>
    )
}
