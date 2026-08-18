import type { FC } from 'react'
import { AlertTriangle } from 'lucide-react'

/**
 * Rendered height of {@link StatusErrorBanner} (`px-3 py-1.5 text-xs` + icon) — used to stack a
 * second banner below a first one that's also showing, since both are `position: fixed` and would
 * otherwise paint on top of each other at the same `top-0` coordinates.
 */
export const STATUS_ERROR_BANNER_HEIGHT_PX = 32

type StatusErrorBannerProps = {
    message: string
    retryLabel: string
    onRetry: () => void
    stackOffsetPx?: number
}

export const StatusErrorBanner: FC<StatusErrorBannerProps> = ({ message, retryLabel, onRetry, stackOffsetPx = 0 }) => (
    <div
        className='bg-status-error/15 text-status-error fixed inset-x-0 z-50 flex items-center gap-2 px-3 py-1.5 text-xs'
        style={{ top: stackOffsetPx }}>
        <AlertTriangle className='size-3.5 shrink-0' />
        <span className='flex-1'>{message}</span>
        <button type='button' onClick={onRetry} className='underline'>
            {retryLabel}
        </button>
    </div>
)
