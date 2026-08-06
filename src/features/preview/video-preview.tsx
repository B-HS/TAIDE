import type { FC } from 'react'

export type VideoPreviewProps = {
    src: string
}

export const VideoPreview: FC<VideoPreviewProps> = ({ src }) => (
    <div className='bg-editor-background flex h-full w-full items-center justify-center p-4'>
        <video src={src} controls className='max-h-full max-w-full' />
    </div>
)
