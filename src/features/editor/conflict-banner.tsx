import type { FC } from 'react'
import { AlertTriangle, Info } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'

export type ConflictBannerVariant = 'changedOnDisk' | 'mirrorRestored' | 'mirrorRestoredConflict'

export type ConflictBannerProps = {
    variant?: ConflictBannerVariant
    onViewDisk: () => void
    onKeepMine: () => void
    onDismiss?: () => void
}

const MESSAGE_KEY_BY_VARIANT: Record<ConflictBannerVariant, string> = {
    changedOnDisk: 'editor.changedOnDisk',
    mirrorRestored: 'editor.mirrorRestored',
    mirrorRestoredConflict: 'editor.mirrorRestoredConflict',
}

const CLASS_NAME_BY_VARIANT: Record<ConflictBannerVariant, string> = {
    changedOnDisk: 'bg-status-error/15 text-status-error',
    mirrorRestoredConflict: 'bg-status-error/15 text-status-error',
    mirrorRestored: 'bg-status-warning/15 text-status-warning',
}

const ICON_BY_VARIANT: Record<ConflictBannerVariant, typeof AlertTriangle> = {
    changedOnDisk: AlertTriangle,
    mirrorRestoredConflict: AlertTriangle,
    mirrorRestored: Info,
}

/**
 * A conflict/notice strip shown above the editor. `changedOnDisk` and `mirrorRestoredConflict`
 * present the same view-disk-content/keep-mine choice (the latter fires when a hot-exit restore
 * finds the disk newer than the mirror's baseline); `mirrorRestored` is a lighter, dismissable
 * notice for a non-conflicting restore of unsaved edits from the last session.
 */
export const ConflictBanner: FC<ConflictBannerProps> = ({ variant = 'changedOnDisk', onViewDisk, onKeepMine, onDismiss }) => {
    const { t } = useTranslation()
    const Icon = ICON_BY_VARIANT[variant]
    const showChoiceActions = variant !== 'mirrorRestored'

    return (
        <div className={cn('flex shrink-0 items-center gap-2 px-3 py-1.5 text-xs', CLASS_NAME_BY_VARIANT[variant])}>
            <Icon className='size-3.5 shrink-0' />
            <span className='flex-1'>{t(MESSAGE_KEY_BY_VARIANT[variant])}</span>
            {showChoiceActions ? (
                <>
                    <Button type='button' variant='outline' size='xs' onClick={onViewDisk}>
                        {t('editor.viewDiskContent')}
                    </Button>
                    <Button type='button' variant='ghost' size='xs' onClick={onKeepMine}>
                        {t('editor.keepMine')}
                    </Button>
                </>
            ) : (
                <Button type='button' variant='ghost' size='xs' onClick={onDismiss}>
                    {t('common.close')}
                </Button>
            )}
        </div>
    )
}
