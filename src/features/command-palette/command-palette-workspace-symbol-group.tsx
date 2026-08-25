import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { Hash } from 'lucide-react'
import { fuzzyMatch } from '@shared/lib/fuzzy-match'
import type { NormalizedWorkspaceSymbol } from '@shared/lib/lsp/adapters/workspace-symbol'
import { CommandGroup, CommandItem } from '@shared/ui/command'
import { HighlightedText } from '@features/command-palette/highlighted-text'

type CommandPaletteWorkspaceSymbolGroupProps = {
    symbols: NormalizedWorkspaceSymbol[]
    searchTerm: string
    onSelectSymbol: (symbol: NormalizedWorkspaceSymbol) => void
}

/**
 * `workspaceSymbolSearch.search()` resolves through the LSP server (contract §3.4 — 2026-08-20
 * palette-ux carryover), so unlike `commands`/`symbol` mode's `fuzzyFilter` results, `symbols` here
 * carries no `FuzzyMatch.indices` to drive `HighlightedText`. Re-running the same local `fuzzyMatch`
 * against `searchTerm.trim()` here recovers an index set for the common case (the server matched the
 * same substring/subsequence our fuzzy matcher would) — trimmed because `useWorkspaceSymbolSearch`
 * queries the LSP server with the trimmed term, and an un-trimmed re-match against a trailing-space
 * query would fail to match every symbol, silently dropping every highlight. This is index recovery
 * only, without touching the LSP query or result set itself. When the server ranked a symbol by
 * criteria our local greedy matcher does not reproduce
 * (e.g. it matched on `containerName` rather than `name`, or used a scoring rule ours does not),
 * `fuzzyMatch` returns `null` and the symbol falls back to plain, unhighlighted text — the exact
 * pre-existing rendering — rather than hiding or mis-highlighting a result the server legitimately
 * returned.
 */
export const CommandPaletteWorkspaceSymbolGroup: FC<CommandPaletteWorkspaceSymbolGroupProps> = ({ symbols, searchTerm, onSelectSymbol }) => {
    const { t } = useTranslation()

    return (
        <CommandGroup heading={t('palette.workspaceSymbols')}>
            {symbols.map((symbol, index) => {
                const match = fuzzyMatch(searchTerm.trim(), symbol.name)
                return (
                    <CommandItem key={`${symbol.path}:${symbol.line}:${symbol.column}:${index}`} onSelect={() => onSelectSymbol(symbol)}>
                        <Hash className='size-4' />
                        <span className='truncate'>{match ? <HighlightedText text={symbol.name} indices={match.indices} /> : symbol.name}</span>
                        {symbol.containerName && <span className='truncate text-xs text-muted-foreground'>{symbol.containerName}</span>}
                    </CommandItem>
                )
            })}
        </CommandGroup>
    )
}
