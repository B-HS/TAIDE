import type { FC } from 'react'
import type { languages } from 'monaco-editor'
import { Box, Braces, Circle, Component, File, Hash, Package, Parentheses, SquareFunction, Variable } from 'lucide-react'
import { monaco } from '@shared/lib/monaco/setup'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'

const SYMBOL_KIND_ICON = {
    [monaco.languages.SymbolKind.File]: File,
    [monaco.languages.SymbolKind.Module]: Package,
    [monaco.languages.SymbolKind.Namespace]: Package,
    [monaco.languages.SymbolKind.Package]: Package,
    [monaco.languages.SymbolKind.Class]: Box,
    [monaco.languages.SymbolKind.Struct]: Box,
    [monaco.languages.SymbolKind.Interface]: Component,
    [monaco.languages.SymbolKind.Enum]: Braces,
    [monaco.languages.SymbolKind.EnumMember]: Hash,
    [monaco.languages.SymbolKind.Constant]: Hash,
    [monaco.languages.SymbolKind.Constructor]: Parentheses,
    [monaco.languages.SymbolKind.Method]: SquareFunction,
    [monaco.languages.SymbolKind.Function]: SquareFunction,
    [monaco.languages.SymbolKind.Property]: Variable,
    [monaco.languages.SymbolKind.Field]: Variable,
    [monaco.languages.SymbolKind.Variable]: Variable,
} as const

const SYMBOL_ROW_DEPTH_INDENT_PX = 16
const SYMBOL_ROW_BASE_INDENT_PX = 8

type OutlineSymbolRowProps = {
    symbol: languages.DocumentSymbol
    depth: number
    onSelect: (symbol: languages.DocumentSymbol) => void
}

export const OutlineSymbolRow: FC<OutlineSymbolRowProps> = ({ symbol, depth, onSelect }) => {
    const Icon = SYMBOL_KIND_ICON[symbol.kind as keyof typeof SYMBOL_KIND_ICON] ?? Circle

    return (
        <div>
            <div
                role='button'
                tabIndex={0}
                onClick={() => onSelect(symbol)}
                onKeyDown={createActivationKeyDownHandler(() => onSelect(symbol))}
                style={{ paddingLeft: SYMBOL_ROW_BASE_INDENT_PX + depth * SYMBOL_ROW_DEPTH_INDENT_PX }}
                className='hover:bg-explorer-item-hover flex cursor-default items-center gap-1.5 py-0.5 pr-2 text-xs select-none'>
                <Icon className='text-app-sidebar-icon-default size-3.5 shrink-0' />
                <span className='truncate'>{symbol.name}</span>
                {symbol.detail && <span className='text-app-sidebar-icon-default truncate'>{symbol.detail}</span>}
            </div>
            {symbol.children?.map((child, index) => (
                <OutlineSymbolRow key={`${child.name}-${index}`} symbol={child} depth={depth + 1} onSelect={onSelect} />
            ))}
        </div>
    )
}
