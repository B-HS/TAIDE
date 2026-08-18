import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'
import { useUninstallPlugin } from '@entities/plugin/plugin.query'
import type { PendingPluginUninstall } from '@features/plugin/plugin-list-body'

type PluginUninstallDialogProps = {
    pending: PendingPluginUninstall | null
    onOpenChange: (open: boolean) => void
}

export const PluginUninstallDialog: FC<PluginUninstallDialogProps> = ({ pending, onOpenChange }) => {
    const { t } = useTranslation()
    const { mutate: uninstallPlugin, isPending } = useUninstallPlugin()

    const handleConfirm = () => {
        if (!pending) return
        uninstallPlugin(pending.pluginId, {
            onSuccess: () => {
                toast.success(t('settings.pluginUninstallSuccess'))
                onOpenChange(false)
            },
            onError: (error) => toast.error(error.message || t('settings.pluginUninstallFailed')),
        })
    }

    return (
        <AlertDialog open={pending !== null} onOpenChange={onOpenChange}>
            <AlertDialogContent size='sm'>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t('settings.pluginUninstallConfirmTitle', { name: pending?.pluginName ?? '' })}</AlertDialogTitle>
                    <AlertDialogDescription>{t('settings.pluginUninstallConfirmDescription')}</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                    <AlertDialogAction variant='destructive' disabled={isPending} onClick={handleConfirm}>
                        {t('settings.pluginUninstallButton')}
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}
