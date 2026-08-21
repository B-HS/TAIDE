import type { FC } from 'react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { syncStatusQueryOptions, useConnectSync, useDisconnectSync, useDownloadSync, useUploadSync } from '@entities/sync/sync.query'
import { SettingsSection } from '@features/settings/settings-section'
import { SyncConflictDialog } from '@features/settings/sync-conflict-dialog'
import { SyncSection } from '@features/settings/sync-section'

type SettingsSyncSectionProps = {
    id: string
}

export const SettingsSyncSection: FC<SettingsSyncSectionProps> = ({ id }) => {
    const [isSyncConflictOpen, setIsSyncConflictOpen] = useState(false)

    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: syncStatus } = useQuery(syncStatusQueryOptions())
    const { mutate: connectSync, isPending: isConnectingSync } = useConnectSync()
    const { mutate: disconnectSync, isPending: isDisconnectingSync } = useDisconnectSync()
    const { mutate: uploadSync, isPending: isUploadingSync } = useUploadSync()
    const { mutate: downloadSync, isPending: isDownloadingSync } = useDownloadSync()

    const { t } = useTranslation()

    const handleConnectSync = (pat: string) => connectSync(pat, { onError: () => toast.error(t('settings.syncConnectFailed')) })
    const handleDisconnectSync = () =>
        disconnectSync(undefined, {
            onSuccess: () => toast.success(t('settings.syncDisconnected')),
            onError: () => toast.error(t('settings.syncDisconnectFailed')),
        })
    const handleUploadSync = () =>
        uploadSync(undefined, {
            onSuccess: () => toast.success(t('settings.syncUploadSuccess')),
            onError: () => toast.error(t('settings.syncUploadFailed')),
        })
    const handleDownloadSync = () =>
        downloadSync(false, {
            onSuccess: (result) => (result.kind === 'conflict' ? setIsSyncConflictOpen(true) : toast.success(t('settings.syncDownloadSuccess'))),
            onError: () => toast.error(t('settings.syncDownloadFailed')),
        })
    const handleSyncConflictKeepLocal = () => {
        setIsSyncConflictOpen(false)
        handleUploadSync()
    }
    const handleSyncConflictPullRemote = () => {
        setIsSyncConflictOpen(false)
        downloadSync(true, {
            onSuccess: () => toast.success(t('settings.syncDownloadSuccess')),
            onError: () => toast.error(t('settings.syncDownloadFailed')),
        })
    }

    return (
        <>
            <SettingsSection id={id} title={t('settings.syncSectionTitle')}>
                <SyncSection
                    status={syncStatus}
                    gistId={settings?.syncGistId ?? null}
                    connecting={isConnectingSync}
                    disconnecting={isDisconnectingSync}
                    uploading={isUploadingSync}
                    downloading={isDownloadingSync}
                    onConnect={handleConnectSync}
                    onDisconnect={handleDisconnectSync}
                    onUpload={handleUploadSync}
                    onDownload={handleDownloadSync}
                />
            </SettingsSection>

            <SyncConflictDialog
                open={isSyncConflictOpen}
                onCancel={() => setIsSyncConflictOpen(false)}
                onKeepLocal={handleSyncConflictKeepLocal}
                onPullRemote={handleSyncConflictPullRemote}
            />
        </>
    )
}
