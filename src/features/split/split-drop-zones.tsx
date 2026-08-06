import type { FC, ReactNode } from 'react'
import { cn } from '@shared/lib/cn'

export const DROP_EDGES = ['left', 'right', 'top', 'bottom', 'center'] as const

export type DropEdgeName = (typeof DROP_EDGES)[number]

const EDGE_CLASS: Record<DropEdgeName, string> = {
    left: 'left-0 top-0 h-full w-1/4',
    right: 'right-0 top-0 h-full w-1/4',
    top: 'left-0 top-0 h-1/4 w-full',
    bottom: 'bottom-0 left-0 h-1/4 w-full',
    center: 'left-1/4 top-1/4 h-1/2 w-1/2',
}

const PREVIEW_CLASS: Record<DropEdgeName, string> = {
    left: 'left-0 top-0 h-full w-1/2',
    right: 'right-0 top-0 h-full w-1/2',
    top: 'left-0 top-0 h-1/2 w-full',
    bottom: 'bottom-0 left-0 h-1/2 w-full',
    center: 'inset-0',
}

type SplitDropZonesProps = {
    activeEdge: DropEdgeName | null
    renderZone: (edge: DropEdgeName, className: string) => ReactNode
}

export const SplitDropZones: FC<SplitDropZonesProps> = ({ activeEdge, renderZone }) => (
    <div className='pointer-events-none absolute inset-0 z-20'>
        {activeEdge && <div className={cn('bg-tab-bar-drop-target/25 border-tab-bar-drop-target absolute border', PREVIEW_CLASS[activeEdge])} />}
        {DROP_EDGES.map((edge) => renderZone(edge, cn('pointer-events-auto absolute', EDGE_CLASS[edge])))}
    </div>
)
