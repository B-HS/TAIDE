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

type CloseDirtyTabDialogProps = {
    /** The dirty tabs the pending close would discard, in strip order. Empty while the dialog is closed. */
    dirtyTitles: string[]
    onSave: () => void
    onDiscard: () => void
    onCancel: () => void
}

/**
 * The confirmation `docs/features/tabs.md` §8 has always required for closing a tab with unsaved
 * edits — three answers, because closing is destructive in a way no other tab gesture is: the close
 * clears the file's hot-exit mirror and disposes its monaco buffer, so neither of the app's two
 * recovery routes (hot exit, ⌘⇧T) brings the text back.
 *
 * One dialog covers a whole close request, listing every dirty tab in it, so "Close Others"/"Close
 * All"/⌘K ⌘W ask once instead of once per tab. The caller owns what each answer does; this component
 * is only the question.
 */
export const CloseDirtyTabDialog: FC<CloseDirtyTabDialogProps> = ({ dirtyTitles, onSave, onDiscard, onCancel }) => {
    const { t } = useTranslation()

    return (
        <AlertDialog open={dirtyTitles.length > 0} onOpenChange={(open) => !open && onCancel()}>
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t('tab.confirmCloseDirtyTitle')}</AlertDialogTitle>
                    <AlertDialogDescription>
                        {dirtyTitles.length === 1
                            ? t('tab.confirmCloseDirtyDescription', { title: dirtyTitles[0] })
                            : t('tab.confirmCloseDirtyDescriptionMany', { count: dirtyTitles.length, titles: dirtyTitles.join(', ') })}
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel onClick={onCancel}>{t('common.cancel')}</AlertDialogCancel>
                    <AlertDialogAction variant='destructive' onClick={onDiscard}>
                        {t('tab.confirmCloseDirtyDiscard')}
                    </AlertDialogAction>
                    <AlertDialogAction onClick={onSave}>{t('tab.confirmCloseDirtySave')}</AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}
