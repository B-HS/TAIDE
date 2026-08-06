import type { FC } from 'react'
import type { languages } from 'monaco-editor'
import { ListTree } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { OutlineSymbolRow } from '@features/outline/outline-symbol-row'

type OutlinePanelProps = {
    hasActiveFile: boolean
    symbols: languages.DocumentSymbol[]
    onSelectSymbol: (symbol: languages.DocumentSymbol) => void
}

export const OutlinePanel: FC<OutlinePanelProps> = ({ hasActiveFile, symbols, onSelectSymbol }) => {
    const { t } = useTranslation()

    return (
        <div className='bg-panel-background flex h-full min-h-0 w-full flex-col'>
            <div className='min-h-0 flex-1 overflow-y-auto'>
                {symbols.length === 0 && (
                    <div className='text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-2 px-4 text-center text-xs'>
                        <ListTree className='size-5 opacity-60' />
                        {t(hasActiveFile ? 'outline.empty' : 'outline.noActiveFile')}
                    </div>
                )}
                {symbols.map((symbol, index) => (
                    <OutlineSymbolRow key={`${symbol.name}-${index}`} symbol={symbol} depth={0} onSelect={onSelectSymbol} />
                ))}
            </div>
        </div>
    )
}
