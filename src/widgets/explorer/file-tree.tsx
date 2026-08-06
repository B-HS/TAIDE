import type { FC, KeyboardEvent } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { FileTreeRowItem } from '@features/explorer/file-tree-row'
import { findTypeaheadMatchIndex } from '@shared/lib/typeahead'

const FILE_TREE_ROW_HEIGHT_PX = 22
const FILE_TREE_OVERSCAN = 12
const TYPEAHEAD_RESET_MS = 700

type FileTreeProps = {
    rows: FileTreeRow[]
    onToggleExpand: (row: FileTreeRow) => void
    onOpenPreview: (row: FileTreeRow) => void
    onOpenPinned: (row: FileTreeRow) => void
}

const findParentIndex = (rows: FileTreeRow[], fromIndex: number) => {
    const depth = rows[fromIndex].depth
    for (let index = fromIndex - 1; index >= 0; index -= 1) {
        if (rows[index].depth < depth) return index
    }
    return -1
}

export const FileTree: FC<FileTreeProps> = ({ rows, onToggleExpand, onOpenPreview, onOpenPinned }) => {
    const parentRef = useRef<HTMLDivElement>(null)
    const typeaheadTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

    const [selectedId, setSelectedId] = useState<string | null>(null)
    const [typeaheadBuffer, setTypeaheadBuffer] = useState('')

    const selectedIndex = rows.findIndex((row) => row.id === selectedId)

    const rowVirtualizer = useVirtualizer({
        count: rows.length,
        getScrollElement: () => parentRef.current,
        estimateSize: () => FILE_TREE_ROW_HEIGHT_PX,
        overscan: FILE_TREE_OVERSCAN,
        getItemKey: (index) => rows[index].id,
    })

    const selectByIndex = (index: number) => {
        if (index < 0 || index >= rows.length) return
        setSelectedId(rows[index].id)
        rowVirtualizer.scrollToIndex(index)
    }

    const handleRowClick = (row: FileTreeRow) => {
        setSelectedId(row.id)
        if (row.kind === 'directory') {
            onToggleExpand(row)
            return
        }
        onOpenPreview(row)
    }

    const handleRowDoubleClick = (row: FileTreeRow) => {
        if (row.kind !== 'file') return
        onOpenPinned(row)
    }

    const handleTypeahead = (char: string) => {
        if (typeaheadTimeoutRef.current) clearTimeout(typeaheadTimeoutRef.current)
        const nextBuffer = typeaheadBuffer + char
        const matchIndex = findTypeaheadMatchIndex(rows, nextBuffer, selectedIndex < 0 ? -1 : selectedIndex - 1)
        setTypeaheadBuffer(nextBuffer)
        if (matchIndex >= 0) selectByIndex(matchIndex)
        typeaheadTimeoutRef.current = setTimeout(() => setTypeaheadBuffer(''), TYPEAHEAD_RESET_MS)
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
            if (selectedRow.kind !== 'directory') return
            if (!selectedRow.expanded) {
                onToggleExpand(selectedRow)
                return
            }
            const childRow = rows[selectedIndex + 1]
            if (childRow && childRow.depth === selectedRow.depth + 1) selectByIndex(selectedIndex + 1)
            return
        }
        if (event.key === 'ArrowLeft') {
            event.preventDefault()
            if (selectedRow.kind === 'directory' && selectedRow.expanded) {
                onToggleExpand(selectedRow)
                return
            }
            selectByIndex(findParentIndex(rows, selectedIndex))
            return
        }
        if (event.key === 'Enter') {
            event.preventDefault()
            if (selectedRow.kind === 'directory') {
                onToggleExpand(selectedRow)
                return
            }
            onOpenPreview(selectedRow)
            return
        }
        if (event.key.length === 1 && !event.metaKey && !event.ctrlKey && !event.altKey) {
            handleTypeahead(event.key)
        }
    }

    useEffect(
        () => () => {
            if (typeaheadTimeoutRef.current) clearTimeout(typeaheadTimeoutRef.current)
        },
        [],
    )

    return (
        <div
            ref={parentRef}
            role='tree'
            aria-label='탐색기'
            tabIndex={0}
            onKeyDown={handleKeyDown}
            className='bg-explorer-background h-full w-full overflow-y-auto outline-none'>
            <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
                {rowVirtualizer.getVirtualItems().map((virtualRow) => (
                    <FileTreeRowItem
                        key={virtualRow.key}
                        row={rows[virtualRow.index]}
                        selected={rows[virtualRow.index].id === selectedId}
                        style={{
                            position: 'absolute',
                            top: 0,
                            left: 0,
                            width: '100%',
                            height: virtualRow.size,
                            transform: `translateY(${virtualRow.start}px)`,
                        }}
                        onClick={() => handleRowClick(rows[virtualRow.index])}
                        onDoubleClick={() => handleRowDoubleClick(rows[virtualRow.index])}
                    />
                ))}
            </div>
        </div>
    )
}
