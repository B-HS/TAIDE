import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
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

type EntryDeleteDialogProps = {
    entryName: string | null
    onCancel: () => void
    onConfirm: () => void
}

export const EntryDeleteDialog: FC<EntryDeleteDialogProps> = ({ entryName, onCancel, onConfirm }) => {
    const { t } = useTranslation()
    const isOpen = entryName !== null

    return (
        <AlertDialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t('explorer.deleteConfirmTitle', { name: entryName })}</AlertDialogTitle>
                    <AlertDialogDescription>{t('explorer.deleteConfirmDescription', { name: entryName })}</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                    <AlertDialogAction variant='destructive' onClick={onConfirm}>
                        {t('explorer.delete')}
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}
