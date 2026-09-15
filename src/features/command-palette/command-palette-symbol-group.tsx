import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { Braces } from 'lucide-react'
import type { FlatPaletteSymbol } from '@shared/lib/command-palette-query'
import type { FuzzyRankedItem } from '@shared/lib/fuzzy-match'
import { CommandGroup, CommandItem } from '@shared/ui/command'
import { HighlightedText } from '@features/command-palette/highlighted-text'

type CommandPaletteSymbolGroupProps = {
    symbols: FuzzyRankedItem<FlatPaletteSymbol>[]
    onSelectSymbol: (symbol: FlatPaletteSymbol) => void
}

/**
 * Renders `FuzzyRankedItem.label` rather than `item.name`: the label is the NFC-normalized name
 * `fuzzyFilter` matched, and `match.indices` are offsets into that string alone.
 */
export const CommandPaletteSymbolGroup: FC<CommandPaletteSymbolGroupProps> = ({ symbols, onSelectSymbol }) => {
    const { t } = useTranslation()

    return (
        <CommandGroup heading={t('palette.symbols')}>
            {symbols.map(({ item, match, label }) => (
                <CommandItem key={`${item.containerLabel}/${item.name}/${item.selectionRange.startLineNumber}`} onSelect={() => onSelectSymbol(item)}>
                    <Braces className='size-4' />
                    <span className='truncate'>
                        <HighlightedText text={label} indices={match.indices} />
                    </span>
                    {item.containerLabel && <span className='truncate text-xs text-muted-foreground'>{item.containerLabel}</span>}
                </CommandItem>
            ))}
        </CommandGroup>
    )
}
