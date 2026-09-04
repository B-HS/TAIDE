import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { File, Loader2 } from 'lucide-react'
import type { FuzzyRankedItem } from '@shared/lib/fuzzy-match'
import { splitFileMatchForDisplay } from '@shared/lib/command-palette-file-match'
import { CommandGroup, CommandItem } from '@shared/ui/command'
import { HighlightedText } from '@features/command-palette/highlighted-text'

type CommandPaletteFilesGroupProps = {
    files: FuzzyRankedItem<string>[]
    isRefreshing: boolean
    toProjectRelativePath: (path: string) => string
    onOpenFile: (path: string) => void
}

/**
 * `isRefreshing` marks the window where the rows on screen are a *previous* `search_list_files`
 * walk and a fresher one is still in flight — the quick-open index is a plain query-cache snapshot,
 * so an in-app create/rename/delete (or a watcher echo) makes the palette re-fetch while it is
 * already open. The rows stay clickable throughout: `layout_open_tab` validates the path before it
 * opens a tab, so a row that went stale fails loudly instead of opening an empty editor, and
 * blocking the list would punish every open on a large project for a case that is already handled.
 */
export const CommandPaletteFilesGroup: FC<CommandPaletteFilesGroupProps> = ({ files, isRefreshing, toProjectRelativePath, onOpenFile }) => {
    const { t } = useTranslation()

    return (
        <CommandGroup
            heading={
                <span className='flex items-center gap-1.5'>
                    {t('palette.files')}
                    {isRefreshing && (
                        <span className='flex items-center gap-1'>
                            <Loader2 className='size-3 animate-spin' />
                            {t('palette.filesRefreshing')}
                        </span>
                    )}
                </span>
            }>
            {files.map(({ item: path, match }) => {
                const { fileName, dirPath, fileNameIndices, dirPathIndices } = splitFileMatchForDisplay(toProjectRelativePath(path), match.indices)
                return (
                    <CommandItem key={path} value={path} onSelect={() => onOpenFile(path)}>
                        <File className='size-4' />
                        <span className='flex min-w-0 flex-col'>
                            <span className='truncate'>
                                <HighlightedText text={fileName} indices={fileNameIndices} />
                            </span>
                            {dirPath !== null && (
                                <span className='truncate text-xs text-muted-foreground'>
                                    <HighlightedText text={dirPath} indices={dirPathIndices} />
                                </span>
                            )}
                        </span>
                    </CommandItem>
                )
            })}
        </CommandGroup>
    )
}
