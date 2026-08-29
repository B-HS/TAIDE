import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { ConflictSides } from '@shared/api/bindings'
import type { resolveDiffViewSettingsProps } from '@shared/lib/diff-view-settings'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { DiffView } from '@features/git/diff-view'

type ConflictCompareDialogProps = {
    sides: ConflictSides | null
    languageId: string
    diffViewSettings: ReturnType<typeof resolveDiffViewSettingsProps>
    onOpenChange: (open: boolean) => void
}

/**
 * Peek view for a conflict region's "ours" vs "theirs" content, opened from
 * {@link ConflictResolutionDialog}'s Compare action. `sides` doubles as the open flag — the caller
 * only sets it once `git_conflict_sides` has resolved, so there is no in-dialog loading state.
 */
export const ConflictCompareDialog: FC<ConflictCompareDialogProps> = ({ sides, languageId, diffViewSettings, onOpenChange }) => {
    const { t } = useTranslation()

    return (
        <Dialog open={sides !== null} onOpenChange={onOpenChange}>
            <DialogContent className='flex h-[70vh] flex-col overflow-hidden sm:max-w-4xl'>
                <DialogHeader>
                    <DialogTitle>{t('git.compareChanges')}</DialogTitle>
                </DialogHeader>
                <div className='min-h-0 flex-1'>
                    {sides && (
                        <DiffView
                            original={sides.ours ?? ''}
                            modified={sides.theirs ?? ''}
                            languageId={languageId}
                            renderSideBySide
                            {...diffViewSettings}
                        />
                    )}
                </div>
            </DialogContent>
        </Dialog>
    )
}
