import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { Button } from '@shared/ui/button'

type CreateTagDialogProps = {
    open: boolean
    targetLabel: string
    isPending: boolean
    onOpenChange: (open: boolean) => void
    onConfirm: (input: { name: string; message: string }) => void
}

const INPUT_CLASS_NAME = 'bg-panel-input-background border-panel-input-border text-app-foreground rounded-sm border px-2 py-1 text-sm outline-none'

export const CreateTagDialog: FC<CreateTagDialogProps> = ({ open, targetLabel, isPending, onOpenChange, onConfirm }) => {
    const { t } = useTranslation()
    const [name, setName] = useState('')
    const [message, setMessage] = useState('')

    const trimmedName = name.trim()

    const handleConfirm = () => {
        if (!trimmedName) return
        onConfirm({ name: trimmedName, message: message.trim() })
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{t('git.createTag')}</DialogTitle>
                    <DialogDescription>{targetLabel}</DialogDescription>
                </DialogHeader>
                <div className='flex flex-col gap-2'>
                    <input
                        autoFocus
                        value={name}
                        onChange={(event) => setName(event.target.value)}
                        placeholder={t('git.tagNamePlaceholder')}
                        className={INPUT_CLASS_NAME}
                    />
                    <input
                        value={message}
                        onChange={(event) => setMessage(event.target.value)}
                        placeholder={t('git.tagMessagePlaceholder')}
                        className={INPUT_CLASS_NAME}
                    />
                </div>
                <DialogFooter>
                    <Button type='button' variant='outline' onClick={() => onOpenChange(false)}>
                        {t('common.cancel')}
                    </Button>
                    <Button type='button' disabled={!trimmedName || isPending} onClick={handleConfirm}>
                        {t('git.createTag')}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}
