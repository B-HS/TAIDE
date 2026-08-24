import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { Hash } from 'lucide-react'
import type { NormalizedWorkspaceSymbol } from '@shared/lib/lsp/adapters/workspace-symbol'
import { CommandGroup, CommandItem } from '@shared/ui/command'

type CommandPaletteWorkspaceSymbolGroupProps = {
    symbols: NormalizedWorkspaceSymbol[]
    onSelectSymbol: (symbol: NormalizedWorkspaceSymbol) => void
}

export const CommandPaletteWorkspaceSymbolGroup: FC<CommandPaletteWorkspaceSymbolGroupProps> = ({ symbols, onSelectSymbol }) => {
    const { t } = useTranslation()

    return (
        <CommandGroup heading={t('palette.workspaceSymbols')}>
            {symbols.map((symbol, index) => (
                <CommandItem key={`${symbol.path}:${symbol.line}:${symbol.column}:${index}`} onSelect={() => onSelectSymbol(symbol)}>
                    <Hash className='size-4' />
                    <span className='truncate'>{symbol.name}</span>
                    {symbol.containerName && <span className='truncate text-xs text-muted-foreground'>{symbol.containerName}</span>}
                </CommandItem>
            ))}
        </CommandGroup>
    )
}
