import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { useQuery } from '@tanstack/react-query'
import { syncStatusQueryOptions } from '@entities/sync/sync.query'
import type { useConnectSync, useDisconnectSync, useDownloadSync, useUploadSync } from '@entities/sync/sync.query'
import { SettingsSection } from '@features/settings/settings-section'
import { SyncConflictDialog } from '@features/settings/sync-conflict-dialog'
import { SyncSection } from '@features/settings/sync-section'
import type { Settings } from '@shared/api/bindings'
import { describeIpcError } from '@shared/lib/ipc-error-message'

type SettingsSyncSectionProps = {
    id: string
    settings: Settings
    isSyncConflictOpen: boolean
    onSyncConflictOpenChange: (open: boolean) => void
    connectSync: ReturnType<typeof useConnectSync>['mutate']
    isConnectingSync: boolean
    disconnectSync: ReturnType<typeof useDisconnectSync>['mutate']
    isDisconnectingSync: boolean
    uploadSync: ReturnType<typeof useUploadSync>['mutate']
    isUploadingSync: boolean
    downloadSync: ReturnType<typeof useDownloadSync>['mutate']
    isDownloadingSync: boolean
}

export const SettingsSyncSection: FC<SettingsSyncSectionProps> = ({
    id,
    settings,
    isSyncConflictOpen,
    onSyncConflictOpenChange,
    connectSync,
    isConnectingSync,
    disconnectSync,
    isDisconnectingSync,
    uploadSync,
    isUploadingSync,
    downloadSync,
    isDownloadingSync,
}) => {
    const { data: syncStatus } = useQuery(syncStatusQueryOptions())

    const { t } = useTranslation()

    const handleConnectSync = (pat: string) =>
        connectSync(pat, { onError: (error) => toast.error(describeIpcError(error) || t('settings.syncConnectFailed')) })
    const handleDisconnectSync = () =>
        disconnectSync(undefined, {
            onSuccess: () => toast.success(t('settings.syncDisconnected')),
            onError: (error) => toast.error(describeIpcError(error) || t('settings.syncDisconnectFailed')),
        })
    const handleUploadSync = () =>
        uploadSync(undefined, {
            onSuccess: () => toast.success(t('settings.syncUploadSuccess')),
            onError: () => toast.error(t('settings.syncUploadFailed')),
        })
    const handleDownloadSync = () =>
        downloadSync(false, {
            onSuccess: (result) => (result.kind === 'conflict' ? onSyncConflictOpenChange(true) : toast.success(t('settings.syncDownloadSuccess'))),
            onError: () => toast.error(t('settings.syncDownloadFailed')),
        })
    const handleSyncConflictKeepLocal = () => {
        onSyncConflictOpenChange(false)
        handleUploadSync()
    }
    const handleSyncConflictPullRemote = () => {
        onSyncConflictOpenChange(false)
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
                    gistId={settings.syncGistId ?? null}
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
                onCancel={() => onSyncConflictOpenChange(false)}
                onKeepLocal={handleSyncConflictKeepLocal}
                onPullRemote={handleSyncConflictPullRemote}
            />
        </>
    )
}
