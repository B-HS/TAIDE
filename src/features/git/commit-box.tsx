import type { FC, KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Loader2, Sparkles } from 'lucide-react'
import { IS_MAC } from '@shared/constants/platform'
import { Button } from '@shared/ui/button'
import { IconButton } from '@shared/ui/icon-button'

type CommitBoxProps = {
    message: string
    onMessageChange: (value: string) => void
    onCommit: () => void
    isCommitting: boolean
    onGenerateCommitMessage: () => void
    isGeneratingCommitMessage: boolean
    canGenerateCommitMessage: boolean
}

const COMMIT_TEXTAREA_ROWS = 3

export const CommitBox: FC<CommitBoxProps> = ({
    message,
    onMessageChange,
    onCommit,
    isCommitting,
    onGenerateCommitMessage,
    isGeneratingCommitMessage,
    canGenerateCommitMessage,
}) => {
    const { t } = useTranslation()
    const isMessageEmpty = message.trim().length === 0

    const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
        const usesModKey = IS_MAC ? event.metaKey : event.ctrlKey
        if (event.key !== 'Enter' || !usesModKey) return
        event.preventDefault()
        if (!isMessageEmpty && !isCommitting) onCommit()
    }

    const resolveGenerateCommitMessageLabel = () => {
        if (isGeneratingCommitMessage) return t('git.generatingCommitMessage')
        if (canGenerateCommitMessage) return t('git.generateCommitMessage')
        return t('git.noStagedChangesForCommitMessage')
    }

    return (
        <div className='border-app-border flex shrink-0 flex-col gap-1.5 border-b px-2 py-1.5'>
            <div className='relative'>
                <textarea
                    value={message}
                    onChange={(event) => onMessageChange(event.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder={t('git.commitMessagePlaceholder')}
                    rows={COMMIT_TEXTAREA_ROWS}
                    className='bg-panel-input-background border-panel-input-border focus:border-app-focus-border w-full resize-none rounded-sm border py-1.5 pr-7 pl-2 text-xs outline-none'
                />
                <IconButton
                    label={resolveGenerateCommitMessageLabel()}
                    icon={isGeneratingCommitMessage ? <Loader2 className='size-3.5 animate-spin' /> : <Sparkles className='size-3.5' />}
                    disabled={!isGeneratingCommitMessage && !canGenerateCommitMessage}
                    onClick={onGenerateCommitMessage}
                    side='left'
                    containerClassName='absolute top-1.5 right-1.5'
                    className='hover:bg-explorer-item-hover flex size-5 shrink-0 items-center justify-center rounded-sm disabled:opacity-50'
                />
            </div>
            <Button size='sm' disabled={isMessageEmpty || isCommitting} onClick={onCommit}>
                {isCommitting ? t('git.committing') : t('git.commit')}
            </Button>
        </div>
    )
}
