import type { FC } from 'react'
import { Music } from 'lucide-react'

export type AudioPreviewProps = {
    src: string
    fileName: string
}

export const AudioPreview: FC<AudioPreviewProps> = ({ src, fileName }) => (
    <div className='bg-editor-background flex h-full w-full flex-col items-center justify-center gap-4 p-4'>
        <Music className='text-app-sidebar-icon-default size-10' />
        <span className='text-editor-foreground max-w-md truncate text-sm'>{fileName}</span>
        <audio src={src} controls className='w-full max-w-md' />
    </div>
)
