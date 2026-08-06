import type { FC } from 'react'
import { FolderInput } from 'lucide-react'

type DragDropOverlayProps = {
    visible: boolean
    label: string
}

export const DragDropOverlay: FC<DragDropOverlayProps> = ({ visible, label }) =>
    visible ? (
        <div className='bg-app-background/85 border-app-accent pointer-events-none absolute inset-0 z-50 flex flex-col items-center justify-center gap-3 border-4 border-dashed'>
            <FolderInput className='text-app-accent size-10' />
            <span className='text-app-foreground text-sm font-medium'>{label}</span>
        </div>
    ) : null
