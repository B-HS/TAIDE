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
import type { ConflictRegion } from '@features/git/conflict-marker'

type ConflictResolutionDialogProps = {
    region: ConflictRegion | null
    onCancel: () => void
    onAcceptCurrent: () => void
    onAcceptIncoming: () => void
    onAcceptBoth: () => void
    onCompare: () => void
}

export const ConflictResolutionDialog: FC<ConflictResolutionDialogProps> = ({
    region,
    onCancel,
    onAcceptCurrent,
    onAcceptIncoming,
    onAcceptBoth,
    onCompare,
}) => {
    const { t } = useTranslation()
    const isOpen = region !== null

    return (
        <AlertDialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t('git.resolveConflictTitle')}</AlertDialogTitle>
                    <AlertDialogDescription>
                        {t('git.resolveConflictDescription', { start: region?.startLine, end: region?.endLine })}
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                    <AlertDialogAction variant='outline' onClick={onCompare}>
                        {t('git.compareChanges')}
                    </AlertDialogAction>
                    <AlertDialogAction variant='outline' onClick={onAcceptBoth}>
                        {t('git.acceptBothChanges')}
                    </AlertDialogAction>
                    <AlertDialogAction variant='outline' onClick={onAcceptIncoming}>
                        {t('git.acceptIncomingChange')}
                    </AlertDialogAction>
                    <AlertDialogAction onClick={onAcceptCurrent}>{t('git.acceptCurrentChange')}</AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}
