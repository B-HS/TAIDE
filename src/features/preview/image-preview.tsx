import type { FC } from 'react'
import { ScrollContainer } from '@shared/scroll/scroll-container'

export type ImagePreviewProps = {
    src: string
    alt: string
}

export const ImagePreview: FC<ImagePreviewProps> = ({ src, alt }) => (
    <ScrollContainer className='bg-editor-background h-full w-full' viewportClassName='flex items-center justify-center p-4' orientation='both'>
        <img src={src} alt={alt} className='max-h-full max-w-full object-contain' />
    </ScrollContainer>
)
