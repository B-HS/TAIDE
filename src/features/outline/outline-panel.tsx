import type { FC, KeyboardEvent } from 'react'
import { useRef, useState } from 'react'
import type { languages } from 'monaco-editor'
import { ListTree } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useVirtualizer } from '@tanstack/react-virtual'
import type { OutlineRow } from '@features/outline/outline-rows'
import { buildOutlineRows, findOutlineParentIndex, OUTLINE_ROW_HEIGHT_PX } from '@features/outline/outline-rows'
import { OutlineSymbolRow } from '@features/outline/outline-symbol-row'
import { toggleInSet } from '@shared/lib/set'
import { OverlayScrollbar } from '@shared/scroll/overlay-scrollbar'

const OUTLINE_OVERSCAN = 12

type OutlinePanelProps = {
    hasActiveFile: boolean
    symbols: languages.DocumentSymbol[]
    onSelectSymbol: (symbol: languages.DocumentSymbol) => void
}

/**
 * Owns its own scroll viewport rather than living inside a `ScrollContainer`: the virtualizer needs
 * the scrolling element to measure against, and only the rows inside that window are mounted. The
 * panel used to render the whole symbol tree — every node, with no collapsing — and re-render it
 * 400ms after each edit when the document symbols are re-requested (research 3a M2).
 *
 * Focus lives on the tree container, not on the rows, which is what keeps the outline usable from
 * the keyboard once off-screen rows stop existing in the DOM: ArrowUp/ArrowDown move the selection
 * (scrolling it into view), ArrowRight/ArrowLeft expand and collapse, Enter reveals. It is the same
 * roving shape `file-tree.tsx` uses.
 */
export const OutlinePanel: FC<OutlinePanelProps> = ({ hasActiveFile, symbols, onSelectSymbol }) => {
    const viewportRef = useRef<HTMLDivElement>(null)

    const [collapsedIds, setCollapsedIds] = useState<Set<string>>(new Set())
    const [selectedId, setSelectedId] = useState<string | null>(null)
    const [isContainerFocused, setIsContainerFocused] = useState(false)

    const rows = buildOutlineRows(symbols, collapsedIds)
    const selectedIndex = rows.findIndex((row) => row.id === selectedId)

    const rowVirtualizer = useVirtualizer({
        count: rows.length,
        getScrollElement: () => viewportRef.current,
        estimateSize: () => OUTLINE_ROW_HEIGHT_PX,
        overscan: OUTLINE_OVERSCAN,
        getItemKey: (index) => rows[index].id,
    })

    const selectByIndex = (index: number) => {
        if (index < 0 || index >= rows.length) return
        setSelectedId(rows[index].id)
        rowVirtualizer.scrollToIndex(index)
    }

    const toggleRow = (row: OutlineRow) => setCollapsedIds((current) => toggleInSet(current, row.id))

    const handleRowClick = (row: OutlineRow) => {
        setSelectedId(row.id)
        onSelectSymbol(row.symbol)
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key === 'ArrowDown') {
            event.preventDefault()
            selectByIndex(selectedIndex < 0 ? 0 : selectedIndex + 1)
            return
        }
        if (event.key === 'ArrowUp') {
            event.preventDefault()
            selectByIndex(selectedIndex < 0 ? rows.length - 1 : selectedIndex - 1)
            return
        }
        if (selectedIndex < 0) return
        const selectedRow = rows[selectedIndex]

        if (event.key === 'ArrowRight') {
            event.preventDefault()
            if (!selectedRow.hasChildren) return
            if (selectedRow.collapsed) {
                toggleRow(selectedRow)
                return
            }
            selectByIndex(selectedIndex + 1)
            return
        }
        if (event.key === 'ArrowLeft') {
            event.preventDefault()
            if (selectedRow.hasChildren && !selectedRow.collapsed) {
                toggleRow(selectedRow)
                return
            }
            selectByIndex(findOutlineParentIndex(rows, selectedIndex))
            return
        }
        if (event.key === 'Enter') {
            event.preventDefault()
            onSelectSymbol(selectedRow.symbol)
        }
    }

    const { t } = useTranslation()

    return (
        <div className='bg-panel-background flex h-full min-h-0 w-full flex-col'>
            {rows.length === 0 ? (
                <div className='text-app-sidebar-icon-default flex min-h-0 flex-1 flex-col items-center justify-center gap-2 px-4 text-center text-xs'>
                    <ListTree className='size-5 opacity-60' />
                    {t(hasActiveFile ? 'outline.empty' : 'outline.noActiveFile')}
                </div>
            ) : (
                <div className='relative min-h-0 flex-1'>
                    <div
                        ref={viewportRef}
                        role='tree'
                        aria-label={t('outline.title')}
                        tabIndex={0}
                        onKeyDown={handleKeyDown}
                        onFocus={() => setIsContainerFocused(true)}
                        onBlur={() => setIsContainerFocused(false)}
                        className='scrollbar-hidden h-full w-full overflow-x-hidden overflow-y-auto outline-none'>
                        <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
                            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                                const row = rows[virtualRow.index]
                                const rowStyle = {
                                    position: 'absolute' as const,
                                    top: 0,
                                    left: 0,
                                    width: '100%',
                                    height: virtualRow.size,
                                    transform: `translateY(${virtualRow.start}px)`,
                                }

                                return (
                                    <OutlineSymbolRow
                                        key={virtualRow.key}
                                        row={row}
                                        selected={row.id === selectedId}
                                        focused={row.id === selectedId && isContainerFocused}
                                        style={rowStyle}
                                        onSelect={() => handleRowClick(row)}
                                        onToggle={() => toggleRow(row)}
                                    />
                                )
                            })}
                        </div>
                    </div>
                    <OverlayScrollbar viewportRef={viewportRef} orientation='vertical' />
                </div>
            )}
        </div>
    )
}
