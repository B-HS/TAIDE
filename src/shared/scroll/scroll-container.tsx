import type { FC, PropsWithChildren, RefObject } from 'react'
import { useRef } from 'react'
import { cn } from '@shared/lib/cn'
import { OverlayScrollbar } from '@shared/scroll/overlay-scrollbar'

export type ScrollContainerOrientation = 'vertical' | 'horizontal' | 'both'

export type ScrollContainerProps = PropsWithChildren<{
    className?: string
    viewportClassName?: string
    viewportRef?: RefObject<HTMLDivElement | null>
    orientation?: ScrollContainerOrientation
    verticalTrackClassName?: string
    horizontalTrackClassName?: string
}>

export const ScrollContainer: FC<ScrollContainerProps> = ({
    children,
    className,
    viewportClassName,
    viewportRef,
    orientation = 'vertical',
    verticalTrackClassName,
    horizontalTrackClassName,
}) => {
    const internalViewportRef = useRef<HTMLDivElement>(null)
    const resolvedViewportRef = viewportRef ?? internalViewportRef
    const showVerticalTrack = orientation === 'vertical' || orientation === 'both'
    const showHorizontalTrack = orientation === 'horizontal' || orientation === 'both'

    return (
        <div className={cn('relative min-h-0', className)}>
            <div
                ref={resolvedViewportRef}
                className={cn(
                    'scrollbar-hidden h-full w-full',
                    orientation === 'vertical' && 'overflow-y-auto overflow-x-hidden',
                    orientation === 'horizontal' && 'overflow-x-auto overflow-y-hidden',
                    orientation === 'both' && 'overflow-auto',
                    viewportClassName,
                )}>
                {children}
            </div>
            {showVerticalTrack && (
                <OverlayScrollbar
                    viewportRef={resolvedViewportRef}
                    orientation='vertical'
                    trackClassName={cn(orientation === 'both' && 'bottom-2.5', verticalTrackClassName)}
                />
            )}
            {showHorizontalTrack && (
                <OverlayScrollbar
                    viewportRef={resolvedViewportRef}
                    orientation='horizontal'
                    trackClassName={cn(orientation === 'both' && 'right-2.5', horizontalTrackClassName)}
                />
            )}
        </div>
    )
}
