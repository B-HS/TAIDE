import type { FC, ReactNode } from 'react'
import { Button } from '@shared/ui/button'

export type PreviewStatusMessageProps = {
    icon: ReactNode
    message: string
    actionLabel?: string
    onAction?: () => void
}

export const PreviewStatusMessage: FC<PreviewStatusMessageProps> = ({ icon, message, actionLabel, onAction }) => (
    <div className='bg-editor-background text-editor-foreground flex h-full w-full flex-col items-center justify-center gap-3 text-sm'>
        {icon}
        <p>{message}</p>
        {actionLabel && onAction && (
            <Button type='button' variant='outline' size='sm' onClick={onAction}>
                {actionLabel}
            </Button>
        )}
    </div>
)
