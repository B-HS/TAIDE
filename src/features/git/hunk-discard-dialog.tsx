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

type HunkDiscardDialogProps = {
    startLine: number | null
    endLine: number | null
    onCancel: () => void
    onConfirm: () => void
}

export const HunkDiscardDialog: FC<HunkDiscardDialogProps> = ({ startLine, endLine, onCancel, onConfirm }) => {
    const { t } = useTranslation()
    const isOpen = startLine !== null && endLine !== null

    return (
        <AlertDialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t('git.discardHunkTitle')}</AlertDialogTitle>
                    <AlertDialogDescription>{t('git.discardHunkDescription', { start: startLine, end: endLine })}</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                    <AlertDialogAction onClick={onConfirm}>{t('git.discardConfirm')}</AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}
