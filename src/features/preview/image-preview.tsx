import type { FC } from 'react'

export type ImagePreviewProps = {
    src: string
    alt: string
}

export const ImagePreview: FC<ImagePreviewProps> = ({ src, alt }) => (
    <div className='bg-editor-background flex h-full w-full items-center justify-center overflow-auto p-4'>
        <img src={src} alt={alt} className='max-h-full max-w-full object-contain' />
    </div>
)
