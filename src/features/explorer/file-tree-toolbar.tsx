import type { FC } from 'react'
import { ChevronsDownUp, FilePlus, FolderPlus, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

type FileTreeToolbarProps = {
    onNewFile: () => void
    onNewFolder: () => void
    onRefresh: () => void
    onCollapseAll: () => void
}

const TOOLBAR_BUTTON_CLASS =
    'text-app-sidebar-icon-default hover:bg-explorer-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

export const FileTreeToolbar: FC<FileTreeToolbarProps> = ({ onNewFile, onNewFolder, onRefresh, onCollapseAll }) => {
    const { t } = useTranslation()

    return (
        <div className='group-hover/explorer:opacity-100 group-focus-within/explorer:opacity-100 flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity'>
            <Tooltip>
                <TooltipTrigger asChild>
                    <button type='button' aria-label={t('explorer.newFile')} onClick={onNewFile} className={TOOLBAR_BUTTON_CLASS}>
                        <FilePlus className='size-4' />
                    </button>
                </TooltipTrigger>
                <TooltipContent side='bottom'>{t('explorer.newFile')}</TooltipContent>
            </Tooltip>

            <Tooltip>
                <TooltipTrigger asChild>
                    <button type='button' aria-label={t('explorer.newFolder')} onClick={onNewFolder} className={TOOLBAR_BUTTON_CLASS}>
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
    )
}
