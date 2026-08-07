import type { FC } from 'react'
import { Separator } from 'react-resizable-panels'
import { cn } from '@shared/lib/cn'

type PaneSeparatorProps = {
    orientation: 'horizontal' | 'vertical'
    thickness: number
}

export const PaneSeparator: FC<PaneSeparatorProps> = ({ orientation, thickness }) => {
    const isHorizontal = orientation === 'horizontal'

    return (
        <Separator
            className={cn('bg-app-border hover:bg-ring shrink-0 transition-colors', isHorizontal ? 'cursor-col-resize' : 'cursor-row-resize')}
            style={isHorizontal ? { width: thickness } : { height: thickness }}
        />
    )
}
