import { toast } from 'sonner'
import type { AppCommand } from '@shared/lib/command-registry'
import { i18next } from '@shared/i18n/i18n'
import { KEYMAP_CATEGORY } from '@shared/lib/keymap/keymap-category'
import { downloadSync, uploadSync } from '@entities/sync/sync.ipc'

const runSyncUpload = async () => {
    try {
        await uploadSync()
        toast.success(i18next.t('settings.syncUploadSuccess'))
    } catch {
        toast.error(i18next.t('settings.syncUploadFailed'))
    }
}

/**
 * The conflict toast's "Pull Remote" action re-enters this function instead of calling
 * `downloadSync(true)` directly: that raw call reported nothing at all — no success toast, and a
 * failure (`sync_download` rejects when the gist changed or another sync completed mid-fetch, and
 * both ask the user to retry) surfaced only as an unhandled rejection. The user was left staring at
 * a dismissed dialog with no way to tell whether their settings had just been replaced (audit §4-B
 * D7). Recursion terminates: `sync_download` only returns `conflict` when `force` is false.
 */
const runSyncDownload = async (force: boolean) => {
    try {
        const result = await downloadSync(force)
        if (result.kind === 'conflict') {
            toast.warning(i18next.t('settings.syncConflictTitle'), {
                description: i18next.t('settings.syncConflictDescription'),
                action: { label: i18next.t('settings.syncConflictPullRemote'), onClick: () => void runSyncDownload(true) },
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
    { id: 'sync.downloadNow', titleKey: 'settings.syncDownloadNow', categoryKey: KEYMAP_CATEGORY.SYNC, run: () => runSyncDownload(false) },
]
