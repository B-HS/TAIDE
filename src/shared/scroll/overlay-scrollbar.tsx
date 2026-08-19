import type { FC, RefObject } from 'react'
import { cn } from '@shared/lib/cn'
import type { OverlayScrollbarOrientation } from '@shared/hooks/use-overlay-scrollbar'
import { useOverlayScrollbar } from '@shared/hooks/use-overlay-scrollbar'

export type OverlayScrollbarProps = {
    viewportRef: RefObject<HTMLDivElement | null>
    orientation?: OverlayScrollbarOrientation
    trackClassName?: string
}

export const OverlayScrollbar: FC<OverlayScrollbarProps> = ({ viewportRef, orientation = 'vertical', trackClassName }) => {
    const { trackRef, thumbRef } = useOverlayScrollbar({ viewportRef, orientation })
    const isVertical = orientation === 'vertical'

    return (
        <div
            ref={trackRef}
            role='scrollbar'
            aria-orientation={orientation}
            aria-valuemin={0}
            aria-valuemax={100}
            data-slot='overlay-scrollbar-track'
            className={cn(
                'pointer-events-none absolute z-10 opacity-0 transition-opacity duration-200 data-[scrollable=true]:pointer-events-auto data-[visible=true]:opacity-100',
                isVertical ? 'top-0 right-0 bottom-0 w-2.5' : 'right-0 bottom-0 left-0 h-2.5',
                trackClassName,
            )}>
            <div
                ref={thumbRef}
                data-slot='overlay-scrollbar-thumb'
                className={cn(
                    'bg-scrollbar-thumb hover:bg-scrollbar-thumb-hover absolute touch-none rounded-full transition-colors select-none',
                    isVertical ? 'top-0 right-0 w-full' : 'bottom-0 left-0 h-full',
                )}
            />
        </div>
    )
}
