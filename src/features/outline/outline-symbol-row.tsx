import type { CSSProperties, FC, MouseEvent } from 'react'
import { Box, Braces, ChevronRight, Circle, Component, File, Hash, Package, Parentheses, SquareFunction, Variable } from 'lucide-react'
import { monaco } from '@shared/lib/monaco/setup'
import { cn } from '@shared/lib/cn'
import type { OutlineRow } from '@features/outline/outline-rows'

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
    row: OutlineRow
    selected: boolean
    focused: boolean
    style: CSSProperties
    onSelect: () => void
    onToggle: () => void
}

/**
 * One flattened outline row. The tree walk moved to `outline-rows.ts` so the panel can virtualize:
 * a row that renders its own children cannot be placed by index, and the panel drew the entire
 * symbol tree on every document symbol refresh (research 3a M2).
 *
 * The row is not a tab stop of its own — the panel's tree container owns focus and moves selection
 * with the arrow keys, the same shape the file tree uses. That is what keeps the outline reachable
 * from the keyboard now that rows outside the viewport are not in the DOM at all. The chevron is a
 * redundant pointer affordance for the same collapse the container's ArrowLeft/ArrowRight perform,
 * so it stays out of the accessibility tree instead of announcing itself as a second control.
 */
export const OutlineSymbolRow: FC<OutlineSymbolRowProps> = ({ row, selected, focused, style, onSelect, onToggle }) => {
    const Icon = SYMBOL_KIND_ICON[row.symbol.kind as keyof typeof SYMBOL_KIND_ICON] ?? Circle

    const handleToggleClick = (event: MouseEvent<HTMLButtonElement>) => {
        event.stopPropagation()
        onToggle()
    }

    return (
        <div
            role='treeitem'
            aria-selected={selected}
            aria-expanded={row.hasChildren ? !row.collapsed : undefined}
            onClick={onSelect}
            style={{ ...style, paddingLeft: SYMBOL_ROW_BASE_INDENT_PX + row.depth * SYMBOL_ROW_DEPTH_INDENT_PX }}
            className={cn(
                'hover:bg-explorer-item-hover flex cursor-default items-center gap-1.5 pr-2 text-xs select-none',
                selected && (focused ? 'bg-explorer-item-selected' : 'bg-explorer-item-focused'),
            )}>
            {row.hasChildren ? (
                <button
                    type='button'
                    tabIndex={-1}
                    aria-hidden='true'
                    onClick={handleToggleClick}
                    className='flex size-3 shrink-0 items-center justify-center'>
                    <ChevronRight className={cn('size-3', !row.collapsed && 'rotate-90')} />
                </button>
            ) : (
                <span className='size-3 shrink-0' />
            )}
            <Icon className='text-app-sidebar-icon-default size-3.5 shrink-0' />
            <span className='truncate'>{row.symbol.name}</span>
            {row.symbol.detail && <span className='text-app-sidebar-icon-default truncate'>{row.symbol.detail}</span>}
        </div>
    )
}
