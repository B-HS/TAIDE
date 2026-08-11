import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'

type SyncConflictDialogProps = {
    open: boolean
    onCancel: () => void
    onKeepLocal: () => void
    onPullRemote: () => void
}

export const SyncConflictDialog: FC<SyncConflictDialogProps> = ({ open, onCancel, onKeepLocal, onPullRemote }) => {
    const { t } = useTranslation()

    return (
        <AlertDialog open={open} onOpenChange={(next) => !next && onCancel()}>
            <AlertDialogContent size='sm'>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t('settings.syncConflictTitle')}</AlertDialogTitle>
                    <AlertDialogDescription>{t('settings.syncConflictDescription')}</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogAction variant='outline' onClick={onKeepLocal}>
                        {t('settings.syncConflictKeepLocal')}
                    </AlertDialogAction>
                    <AlertDialogAction variant='default' onClick={onPullRemote}>
                        {t('settings.syncConflictPullRemote')}
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}
