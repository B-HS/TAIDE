import type { FC, RefObject } from 'react'

export type BlameFooterBarProps = {
    textRef: RefObject<HTMLSpanElement | null>
}

export const BlameFooterBar: FC<BlameFooterBarProps> = ({ textRef }) => (
    <div className='bg-editor-blame-background text-editor-blame-foreground border-app-border flex h-6 shrink-0 items-center border-t px-2 text-[11px]'>
        <span ref={textRef} className='min-w-0 truncate' />
    </div>
)
