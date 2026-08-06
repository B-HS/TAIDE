import type { FC } from 'react'
import { AlertTriangle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@shared/ui/button'

export type ConflictBannerProps = {
    onViewDisk: () => void
    onKeepMine: () => void
}

export const ConflictBanner: FC<ConflictBannerProps> = ({ onViewDisk, onKeepMine }) => {
    const { t } = useTranslation()

    return (
        <div className='bg-status-error/15 text-status-error flex shrink-0 items-center gap-2 px-3 py-1.5 text-xs'>
            <AlertTriangle className='size-3.5 shrink-0' />
            <span className='flex-1'>{t('editor.changedOnDisk')}</span>
            <Button type='button' variant='outline' size='xs' onClick={onViewDisk}>
                {t('editor.viewDiskContent')}
            </Button>
            <Button type='button' variant='ghost' size='xs' onClick={onKeepMine}>
                {t('editor.keepMine')}
            </Button>
        </div>
    )
}
