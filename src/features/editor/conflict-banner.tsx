import type { FC } from 'react'
import { AlertTriangle } from 'lucide-react'
import { Button } from '@shared/ui/button'

export type ConflictBannerProps = {
    onViewDisk: () => void
    onKeepMine: () => void
}

export const ConflictBanner: FC<ConflictBannerProps> = ({ onViewDisk, onKeepMine }) => (
    <div className='bg-status-error/15 text-status-error flex shrink-0 items-center gap-2 px-3 py-1.5 text-xs'>
        <AlertTriangle className='size-3.5 shrink-0' />
        <span className='flex-1'>디스크에서 변경됨</span>
        <Button type='button' variant='outline' size='xs' onClick={onViewDisk}>
            디스크 내용 보기
        </Button>
        <Button type='button' variant='ghost' size='xs' onClick={onKeepMine}>
            내 변경 유지
        </Button>
    </div>
)
