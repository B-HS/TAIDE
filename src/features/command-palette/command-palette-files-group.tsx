import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { File } from 'lucide-react'
import type { FuzzyRankedItem } from '@shared/lib/fuzzy-match'
import { splitFileMatchForDisplay } from '@shared/lib/command-palette-file-match'
import { CommandGroup, CommandItem } from '@shared/ui/command'
import { HighlightedText } from '@features/command-palette/highlighted-text'

type CommandPaletteFilesGroupProps = {
    files: FuzzyRankedItem<string>[]
    toProjectRelativePath: (path: string) => string
    onOpenFile: (path: string) => void
}

export const CommandPaletteFilesGroup: FC<CommandPaletteFilesGroupProps> = ({ files, toProjectRelativePath, onOpenFile }) => {
    const { t } = useTranslation()

    return (
        <CommandGroup heading={t('palette.files')}>
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
