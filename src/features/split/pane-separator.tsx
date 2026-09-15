import type { FC } from 'react'
import { Separator } from 'react-resizable-panels'
import { cn } from '@shared/lib/cn'

type PaneSeparatorProps = {
    orientation: 'horizontal' | 'vertical'
    thickness: number
    /** Takes the separator out of flow without unmounting it, so a group can hide one of its divisions (Zen mode, `shell-slot-tree-view.tsx`) while every panel around it stays mounted and registered. */
    hidden?: boolean
}

export const PaneSeparator: FC<PaneSeparatorProps> = ({ orientation, thickness, hidden }) => {
    const isHorizontal = orientation === 'horizontal'

    return (
        <Separator
            hidden={hidden}
            className={cn('bg-app-border hover:bg-ring shrink-0 transition-colors', isHorizontal ? 'cursor-col-resize' : 'cursor-row-resize')}
            style={isHorizontal ? { width: thickness } : { height: thickness }}
        />
    )
}
