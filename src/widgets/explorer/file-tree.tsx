import type { FC, KeyboardEvent } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useTranslation } from 'react-i18next'
import type { FileTreeNodeKind, FileTreeRow } from '@features/explorer/file-tree-row'
import { FileTreeRowItem } from '@features/explorer/file-tree-row'
import { FileTreeDraftRowItem } from '@features/explorer/file-tree-draft-row'
import { findTypeaheadMatchIndex } from '@shared/lib/typeahead'

const FILE_TREE_ROW_HEIGHT_PX = 22
const FILE_TREE_OVERSCAN = 12
const TYPEAHEAD_RESET_MS = 700
const DRAFT_ROW_ID = '__taide_draft__'

export type FileTreeDraft = { kind: FileTreeNodeKind; parentDir: string }

type FileTreeProps = {
    rows: FileTreeRow[]
    draft: FileTreeDraft | null
    draftError: string | null
    selectPathRequest: string | null
    onToggleExpand: (row: FileTreeRow) => void
    onOpenPreview: (row: FileTreeRow) => void
    onOpenPinned: (row: FileTreeRow) => void
    onSelectionChange?: (row: FileTreeRow) => void
    onDraftCommit: (name: string) => void
    onDraftCancel: () => void
    onSelectPathRequestHandled: () => void
}

const findParentIndex = (rows: FileTreeRow[], fromIndex: number) => {
    const depth = rows[fromIndex].depth
    for (let index = fromIndex - 1; index >= 0; index -= 1) {
        if (rows[index].depth < depth) return index
    }
    return -1
}

const buildDisplayRows = (rows: FileTreeRow[], draft: FileTreeDraft | null): FileTreeRow[] => {
    if (!draft) return rows
    const targetRow = rows.find((row) => row.path === draft.parentDir)
    const depth = targetRow ? targetRow.depth + 1 : 0
    const insertIndex = targetRow ? rows.indexOf(targetRow) + 1 : 0
    const draftRow: FileTreeRow = { id: DRAFT_ROW_ID, path: '', name: '', depth, kind: draft.kind, expanded: false, gitStatus: null }
    return [...rows.slice(0, insertIndex), draftRow, ...rows.slice(insertIndex)]
}

export const FileTree: FC<FileTreeProps> = ({
    rows,
    draft,
    draftError,
    selectPathRequest,
    onToggleExpand,
    onOpenPreview,
    onOpenPinned,
    onSelectionChange,
    onDraftCommit,
    onDraftCancel,
    onSelectPathRequestHandled,
}) => {
    const { t } = useTranslation()
    const parentRef = useRef<HTMLDivElement>(null)
    const typeaheadTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

    const [selectedId, setSelectedId] = useState<string | null>(null)
    const [typeaheadBuffer, setTypeaheadBuffer] = useState('')

    const displayRows = buildDisplayRows(rows, draft)
    const selectedIndex = displayRows.findIndex((row) => row.id === selectedId)

    const rowVirtualizer = useVirtualizer({
        count: displayRows.length,
        getScrollElement: () => parentRef.current,
        estimateSize: () => FILE_TREE_ROW_HEIGHT_PX,
        overscan: FILE_TREE_OVERSCAN,
        getItemKey: (index) => displayRows[index].id,
    })

    const selectByIndex = (index: number) => {
        if (index < 0 || index >= displayRows.length) return
        const row = displayRows[index]
        if (row.id === DRAFT_ROW_ID) return
        setSelectedId(row.id)
        onSelectionChange?.(row)
        rowVirtualizer.scrollToIndex(index)
    }

    const handleRowClick = (row: FileTreeRow) => {
        setSelectedId(row.id)
        onSelectionChange?.(row)
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
        const matchIndex = findTypeaheadMatchIndex(displayRows, nextBuffer, selectedIndex < 0 ? -1 : selectedIndex - 1)
        setTypeaheadBuffer(nextBuffer)
        if (matchIndex >= 0) selectByIndex(matchIndex)
        typeaheadTimeoutRef.current = setTimeout(() => setTypeaheadBuffer(''), TYPEAHEAD_RESET_MS)
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (draft) return
        if (event.key === 'ArrowDown') {
            event.preventDefault()
            selectByIndex(selectedIndex < 0 ? 0 : selectedIndex + 1)
            return
        }
        if (event.key === 'ArrowUp') {
            event.preventDefault()
            selectByIndex(selectedIndex < 0 ? displayRows.length - 1 : selectedIndex - 1)
            return
        }
        if (selectedIndex < 0) return
        const selectedRow = displayRows[selectedIndex]

        if (event.key === 'ArrowRight') {
            event.preventDefault()
            if (selectedRow.kind !== 'directory') return
            if (!selectedRow.expanded) {
                onToggleExpand(selectedRow)
                return
            }
            const childRow = displayRows[selectedIndex + 1]
            if (childRow && childRow.depth === selectedRow.depth + 1) selectByIndex(selectedIndex + 1)
            return
        }
        if (event.key === 'ArrowLeft') {
            event.preventDefault()
            if (selectedRow.kind === 'directory' && selectedRow.expanded) {
                onToggleExpand(selectedRow)
                return
            }
            selectByIndex(findParentIndex(displayRows, selectedIndex))
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

    useEffect(() => {
        if (!draft) return
        const insertIndex = displayRows.findIndex((row) => row.id === DRAFT_ROW_ID)
        if (insertIndex < 0) return
        rowVirtualizer.scrollToIndex(insertIndex)
    }, [draft])

    useEffect(() => {
        if (!selectPathRequest) return
        const index = displayRows.findIndex((row) => row.path === selectPathRequest)
        if (index < 0) return
        selectByIndex(index)
        onSelectPathRequestHandled()
    }, [selectPathRequest, rows])

    return (
        <div
            ref={parentRef}
            role='tree'
            aria-label={t('explorer.title')}
            tabIndex={0}
            onKeyDown={handleKeyDown}
            className='bg-explorer-background h-full w-full overflow-y-auto outline-none'>
            <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                    const row = displayRows[virtualRow.index]
                    const rowStyle = {
                        position: 'absolute' as const,
                        top: 0,
                        left: 0,
                        width: '100%',
                        height: virtualRow.size,
                        transform: `translateY(${virtualRow.start}px)`,
                    }

                    if (row.id === DRAFT_ROW_ID) {
                        return (
                            <FileTreeDraftRowItem
                                key={virtualRow.key}
                                depth={row.depth}
                                kind={row.kind}
                                error={draftError}
                                style={rowStyle}
                                onCommit={onDraftCommit}
                                onCancel={onDraftCancel}
                            />
                        )
                    }

                    return (
                        <FileTreeRowItem
                            key={virtualRow.key}
                            row={row}
                            selected={row.id === selectedId}
                            style={rowStyle}
                            onClick={() => handleRowClick(row)}
                            onDoubleClick={() => handleRowDoubleClick(row)}
                        />
                    )
                })}
            </div>
        </div>
    )
}
