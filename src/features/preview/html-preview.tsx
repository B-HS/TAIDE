import type { FC } from 'react'

export type HtmlPreviewProps = {
    src: string
    title: string
}

export const HtmlPreview: FC<HtmlPreviewProps> = ({ src, title }) => (
    <iframe src={src} title={title} sandbox='' className='bg-editor-background h-full w-full border-0' />
)
