import type { FC } from 'react'
import { Separator } from 'react-resizable-panels'
import { cn } from '@shared/lib/cn'

export const SEPARATOR_HIT_AREA_PX = 8

type PaneSeparatorProps = {
    orientation: 'horizontal' | 'vertical'
    thickness: number
}

export const PaneSeparator: FC<PaneSeparatorProps> = ({ orientation, thickness }) => {
    const isHorizontal = orientation === 'horizontal'

    return (
        <Separator
            className={cn('group flex shrink-0 items-center justify-center', isHorizontal ? 'cursor-col-resize' : 'cursor-row-resize')}
            style={isHorizontal ? { width: SEPARATOR_HIT_AREA_PX } : { height: SEPARATOR_HIT_AREA_PX }}>
            <div
                className='bg-app-border group-hover:bg-ring transition-colors'
                style={isHorizontal ? { width: thickness, height: '100%' } : { height: thickness, width: '100%' }}
            />
        </Separator>
    )
}
