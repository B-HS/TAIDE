import type { FC, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import {
    ClipboardPaste,
    Columns2,
    Copy,
    FileDiff,
    FilePlus,
    FolderOpen,
    FolderPlus,
    GitCompare,
    Globe,
    History,
    Pencil,
    Scissors,
    Search,
    SquareTerminal,
    Trash2,
} from 'lucide-react'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { resolvePreviewKind } from '@shared/lib/preview-kind'
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuSub,
    ContextMenuSubContent,
    ContextMenuSubTrigger,
    ContextMenuTrigger,
} from '@shared/ui/context-menu'

type FileTreeContextMenuProps = {
    row: FileTreeRow | null
    children: ReactNode
    canPaste: boolean
    onOpenChange: (open: boolean) => void
    onNewFile: () => void
    onNewFolder: () => void
    onOpenToTheSide: () => void
    onOpenWithEditor: () => void
    onOpenWithPreview: () => void
    onOpenInBrowser: () => void
    onRevealInFinder: () => void
    onOpenInTerminal: () => void
    onFindInFolder: () => void
    onSelectForCompare: () => void
    onCompareWithSelected: () => void
    canCompareWithSelected: boolean
    onFileHistory: () => void
    onCut: () => void
    onCopy: () => void
    onPaste: () => void
    onCopyPath: () => void
    onCopyRelativePath: () => void
    onRename: () => void
    onDelete: () => void
}

export const FileTreeContextMenu: FC<FileTreeContextMenuProps> = ({
    row,
    children,
    canPaste,
    onOpenChange,
    onNewFile,
    onNewFolder,
    onOpenToTheSide,
    onOpenWithEditor,
    onOpenWithPreview,
    onOpenInBrowser,
    onRevealInFinder,
    onOpenInTerminal,
    onFindInFolder,
    onSelectForCompare,
    onCompareWithSelected,
    canCompareWithSelected,
    onFileHistory,
    onCut,
    onCopy,
    onPaste,
    onCopyPath,
    onCopyRelativePath,
    onRename,
    onDelete,
}) => {
    const { t } = useTranslation()
    const isFile = row?.kind === 'file'
    const isDirectory = row?.kind === 'directory'
    const previewKind = row && row.kind === 'file' ? resolvePreviewKind(row.name) : null
    const canOpenWith = previewKind !== null
    const canOpenInBrowser = previewKind === 'html'

    return (
        <ContextMenu onOpenChange={onOpenChange}>
            <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
            <ContextMenuContent>
                <ContextMenuItem onSelect={onNewFile}>
                    <FilePlus className='size-4' />
                    {t('explorer.newFile')}
                </ContextMenuItem>
                <ContextMenuItem onSelect={onNewFolder}>
                    <FolderPlus className='size-4' />
                    {t('explorer.newFolder')}
                </ContextMenuItem>

                {isFile && (
                    <>
                        <ContextMenuSeparator />
                        <ContextMenuItem onSelect={onOpenToTheSide}>
                            <Columns2 className='size-4' />
                            {t('explorer.openToTheSide')}
                        </ContextMenuItem>
                        {canOpenWith && (
                            <ContextMenuSub>
                                <ContextMenuSubTrigger>{t('explorer.openWith')}</ContextMenuSubTrigger>
                                <ContextMenuSubContent>
                                    <ContextMenuItem onSelect={onOpenWithEditor}>{t('explorer.openWithEditor')}</ContextMenuItem>
                                    <ContextMenuItem onSelect={onOpenWithPreview}>{t('explorer.openWithPreview')}</ContextMenuItem>
                                </ContextMenuSubContent>
                            </ContextMenuSub>
                        )}
                        {canOpenInBrowser && (
                            <ContextMenuItem onSelect={onOpenInBrowser}>
                                <Globe className='size-4' />
                                {t('explorer.openInBrowser')}
                            </ContextMenuItem>
                        )}
                        <ContextMenuItem onSelect={onSelectForCompare}>
                            <FileDiff className='size-4' />
                            {t('explorer.selectForCompare')}
                        </ContextMenuItem>
                        <ContextMenuItem disabled={!canCompareWithSelected} onSelect={onCompareWithSelected}>
                            <GitCompare className='size-4' />
                            {t('explorer.compareWithSelected')}
                        </ContextMenuItem>
                        <ContextMenuItem onSelect={onFileHistory}>
                            <History className='size-4' />
                            {t('git.fileHistory')}
                        </ContextMenuItem>
                    </>
                )}

                {row && (
                    <>
                        <ContextMenuSeparator />
                        <ContextMenuItem onSelect={onRevealInFinder}>
                            <FolderOpen className='size-4' />
                            {t('explorer.revealInFinder')}
                        </ContextMenuItem>
                        <ContextMenuItem onSelect={onOpenInTerminal}>
                            <SquareTerminal className='size-4' />
                            {t('explorer.openInTerminal')}
                        </ContextMenuItem>
                        {isDirectory && (
                            <ContextMenuItem onSelect={onFindInFolder}>
                                <Search className='size-4' />
                                {t('explorer.findInFolder')}
                            </ContextMenuItem>
                        )}
                    </>
                )}

                <ContextMenuSeparator />
                {row && (
                    <>
                        <ContextMenuItem onSelect={onCut}>
                            <Scissors className='size-4' />
                            {t('explorer.cut')}
                        </ContextMenuItem>
                        <ContextMenuItem onSelect={onCopy}>
                            <Copy className='size-4' />
                            {t('explorer.copy')}
                        </ContextMenuItem>
                    </>
                )}
                <ContextMenuItem disabled={!canPaste} onSelect={onPaste}>
                    <ClipboardPaste className='size-4' />
                    {t('explorer.paste')}
                </ContextMenuItem>
                {row && (
                    <>
                        <ContextMenuItem onSelect={onCopyPath}>{t('explorer.copyPath')}</ContextMenuItem>
                        <ContextMenuItem onSelect={onCopyRelativePath}>{t('explorer.copyRelativePath')}</ContextMenuItem>
                    </>
                )}

                {row && (
                    <>
                        <ContextMenuSeparator />
                        <ContextMenuItem onSelect={onRename}>
                            <Pencil className='size-4' />
                            {t('explorer.rename')}
                        </ContextMenuItem>
                        <ContextMenuItem variant='destructive' onSelect={onDelete}>
                            <Trash2 className='size-4' />
                            {t('explorer.delete')}
                        </ContextMenuItem>
                    </>
                )}
            </ContextMenuContent>
        </ContextMenu>
    )
}
