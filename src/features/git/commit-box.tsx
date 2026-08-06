import type { FC, KeyboardEvent } from 'react'
import { IS_MAC } from '@shared/constants/platform'
import { Button } from '@shared/ui/button'

type CommitBoxProps = {
    message: string
    onMessageChange: (value: string) => void
    onCommit: () => void
    isCommitting: boolean
}

const COMMIT_TEXTAREA_ROWS = 3

export const CommitBox: FC<CommitBoxProps> = ({ message, onMessageChange, onCommit, isCommitting }) => {
    const isMessageEmpty = message.trim().length === 0

    const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
        const usesModKey = IS_MAC ? event.metaKey : event.ctrlKey
        if (event.key !== 'Enter' || !usesModKey) return
        event.preventDefault()
        if (!isMessageEmpty && !isCommitting) onCommit()
    }

    return (
        <div className='border-app-border flex shrink-0 flex-col gap-1.5 border-b px-2 py-1.5'>
            <textarea
                value={message}
                onChange={(event) => onMessageChange(event.target.value)}
                onKeyDown={handleKeyDown}
                placeholder='커밋 메시지 입력'
                rows={COMMIT_TEXTAREA_ROWS}
                className='bg-panel-input-background border-panel-input-border focus:border-app-focus-border resize-none rounded-sm border px-2 py-1.5 text-xs outline-none'
            />
            <Button size='sm' disabled={isMessageEmpty || isCommitting} onClick={onCommit}>
                {isCommitting ? '커밋 중…' : 'Commit'}
            </Button>
        </div>
    )
}
