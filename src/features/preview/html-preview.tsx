import type { FC } from 'react'

export type HtmlPreviewProps = {
    document: string
    title: string
}

export const HtmlPreview: FC<HtmlPreviewProps> = ({ document, title }) => (
    <iframe srcDoc={document} title={title} sandbox='allow-same-origin' className='bg-editor-background h-full w-full border-0' />
)
