import { toast } from 'sonner'
import type { AppCommand } from '@shared/lib/command-registry'
import { i18next } from '@shared/i18n/i18n'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap-category'
import { downloadSync, uploadSync } from '@entities/sync/sync.ipc'

const runSyncUpload = async () => {
    try {
        await uploadSync()
        toast.success(i18next.t('settings.syncUploadSuccess'))
    } catch {
        toast.error(i18next.t('settings.syncUploadFailed'))
    }
}

const runSyncDownload = async () => {
    try {
        const result = await downloadSync(false)
        if (result.kind === 'conflict') {
            toast.warning(i18next.t('settings.syncConflictTitle'), {
                description: i18next.t('settings.syncConflictDescription'),
                action: { label: i18next.t('settings.syncConflictPullRemote'), onClick: () => void downloadSync(true) },
            })
            return
        }
        toast.success(i18next.t('settings.syncDownloadSuccess'))
    } catch {
        toast.error(i18next.t('settings.syncDownloadFailed'))
    }
}

export const SYNC_COMMANDS: AppCommand[] = [
    { id: 'sync.uploadNow', titleKey: 'settings.syncUploadNow', categoryKey: KEYMAP_CATEGORY.SYNC, run: runSyncUpload },
    { id: 'sync.downloadNow', titleKey: 'settings.syncDownloadNow', categoryKey: KEYMAP_CATEGORY.SYNC, run: runSyncDownload },
]
