import type { FC } from 'react'
import { useRef } from 'react'
import { OverlayScrollbar } from '@shared/scroll/overlay-scrollbar'

const MARKDOWN_PREVIEW_CLASS =
    'scrollbar-hidden bg-editor-background text-editor-foreground h-full w-full overflow-auto px-6 py-4 text-sm ' +
    '[&_a]:text-app-accent [&_a]:underline ' +
    '[&_blockquote]:border-l-2 [&_blockquote]:border-app-border [&_blockquote]:pl-3 [&_blockquote]:opacity-80 ' +
    '[&_code]:rounded [&_code]:bg-app-sidebar-item-hover [&_code]:px-1 [&_code]:py-0.5 ' +
    '[&_h1]:mt-4 [&_h1]:mb-2 [&_h1]:text-2xl [&_h1]:font-semibold ' +
    '[&_h2]:mt-4 [&_h2]:mb-2 [&_h2]:text-xl [&_h2]:font-semibold ' +
    '[&_h3]:mt-3 [&_h3]:mb-2 [&_h3]:text-lg [&_h3]:font-semibold ' +
    '[&_hr]:my-4 [&_hr]:border-app-border ' +
    '[&_img]:max-w-full ' +
    '[&_li]:ml-5 ' +
    '[&_ol]:list-decimal ' +
    '[&_p]:my-2 [&_p]:leading-relaxed ' +
    '[&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-app-sidebar-item-hover [&_pre]:p-3 [&_pre]:my-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 ' +
    '[&_table]:border-collapse ' +
    '[&_td]:border [&_td]:border-app-border [&_td]:px-2 [&_td]:py-1 ' +
    '[&_th]:border [&_th]:border-app-border [&_th]:px-2 [&_th]:py-1 ' +
    '[&_ul]:list-disc'

type MarkdownPreviewProps = {
    html: string
}

export const MarkdownPreview: FC<MarkdownPreviewProps> = ({ html }) => {
    const viewportRef = useRef<HTMLDivElement>(null)

    return (
        <div className='relative h-full w-full'>
            <div ref={viewportRef} className={MARKDOWN_PREVIEW_CLASS} dangerouslySetInnerHTML={{ __html: html }} />
            <OverlayScrollbar viewportRef={viewportRef} orientation='vertical' trackClassName='bottom-2.5' />
            <OverlayScrollbar viewportRef={viewportRef} orientation='horizontal' trackClassName='right-2.5' />
        </div>
    )
}
