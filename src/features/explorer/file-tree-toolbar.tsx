import type { FC, FormEvent } from 'react'
import { useState } from 'react'
import { ChevronsDownUp, FilePlus, FolderPlus, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@shared/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

type NewEntryDialogKind = 'file' | 'folder' | null

type FileTreeToolbarProps = {
    onCreateFile: (name: string) => void
    onCreateFolder: (name: string) => void
    onRefresh: () => void
    onCollapseAll: () => void
}

const TOOLBAR_BUTTON_CLASS =
    'text-app-sidebar-icon-default hover:bg-explorer-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

export const FileTreeToolbar: FC<FileTreeToolbarProps> = ({ onCreateFile, onCreateFolder, onRefresh, onCollapseAll }) => {
    const { t } = useTranslation()
    const [dialogKind, setDialogKind] = useState<NewEntryDialogKind>(null)
    const [name, setName] = useState('')

    const closeDialog = () => {
        setDialogKind(null)
        setName('')
    }

    const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault()
        const trimmedName = name.trim()
        if (!trimmedName) return
        if (dialogKind === 'file') onCreateFile(trimmedName)
        if (dialogKind === 'folder') onCreateFolder(trimmedName)
        closeDialog()
    }

    return (
        <>
            <div className='flex shrink-0 items-center gap-0.5'>
                <Tooltip>
                    <TooltipTrigger asChild>
                        <button
                            type='button'
                            aria-label={t('explorer.newFile')}
                            onClick={() => setDialogKind('file')}
                            className={TOOLBAR_BUTTON_CLASS}>
                            <FilePlus className='size-4' />
                        </button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('explorer.newFile')}</TooltipContent>
                </Tooltip>

                <Tooltip>
                    <TooltipTrigger asChild>
                        <button
                            type='button'
                            aria-label={t('explorer.newFolder')}
                            onClick={() => setDialogKind('folder')}
                            className={TOOLBAR_BUTTON_CLASS}>
                            <FolderPlus className='size-4' />
                        </button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('explorer.newFolder')}</TooltipContent>
                </Tooltip>

                <Tooltip>
                    <TooltipTrigger asChild>
                        <button type='button' aria-label={t('explorer.refresh')} onClick={onRefresh} className={TOOLBAR_BUTTON_CLASS}>
                            <RefreshCw className='size-4' />
                        </button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('explorer.refresh')}</TooltipContent>
                </Tooltip>

                <Tooltip>
                    <TooltipTrigger asChild>
                        <button type='button' aria-label={t('explorer.collapseAll')} onClick={onCollapseAll} className={TOOLBAR_BUTTON_CLASS}>
                            <ChevronsDownUp className='size-4' />
                        </button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('explorer.collapseAll')}</TooltipContent>
                </Tooltip>
            </div>

            <Dialog open={dialogKind !== null} onOpenChange={(open) => !open && closeDialog()}>
                <DialogContent>
                    <form onSubmit={handleSubmit}>
                        <DialogHeader>
                            <DialogTitle>{dialogKind === 'folder' ? t('explorer.newFolder') : t('explorer.newFile')}</DialogTitle>
                        </DialogHeader>
                        <input
                            autoFocus
                            value={name}
                            onChange={(event) => setName(event.target.value)}
                            placeholder={t('explorer.entryNamePlaceholder')}
                            className='bg-panel-input-background border-panel-input-border focus:border-ring text-app-foreground mt-4 w-full rounded-sm border px-2 py-1.5 text-sm outline-none'
                        />
                        <DialogFooter className='mt-4'>
                            <Button type='button' variant='outline' onClick={closeDialog}>
                                {t('common.cancel')}
                            </Button>
                            <Button type='submit' disabled={!name.trim()}>
                                {t('explorer.create')}
                            </Button>
                        </DialogFooter>
                    </form>
                </DialogContent>
            </Dialog>
        </>
    )
}
