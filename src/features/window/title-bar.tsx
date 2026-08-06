import type { FC, PropsWithChildren } from 'react'

export const TitleBar: FC<PropsWithChildren> = ({ children }) => (
    <div data-tauri-drag-region className='bg-app-background text-app-foreground flex h-7 shrink-0 items-center justify-center text-xs select-none'>
        {children}
    </div>
)
