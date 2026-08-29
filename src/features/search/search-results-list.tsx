import type { FC } from 'react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useVirtualizer } from '@tanstack/react-virtual'
import type { SearchResultGroup } from '@entities/search/search-result'
import { SearchMatchRow } from '@features/search/search-match-row'
import { buildSearchResultRows, estimateSearchResultRowHeight } from '@features/search/search-result-rows'
import { cn } from '@shared/lib/cn'
import { toggleInSet } from '@shared/lib/set'
import { FileGroupHeader } from '@shared/ui/file-group-header'
import { OverlayScrollbar } from '@shared/scroll/overlay-scrollbar'

const SEARCH_RESULT_OVERSCAN = 12

type SearchResultsListSelection = {
    excludedPaths: Set<string>
    onToggleSelect: (path: string) => void
}

type SearchResultsListProps = {
    results: SearchResultGroup[]
    onOpenMatch: (path: string, line: number, column: number) => void
    selection?: SearchResultsListSelection
    className?: string
}

/**
 * Owns its own scroll viewport rather than living inside a caller's `ScrollContainer`: the
 * virtualizer needs the scrolling element to measure against, and only rows inside that window are
 * mounted. Every match row and, in Search Editor results, every context line used to be in the DOM
 * simultaneously — up to the backend's 10,000-match cap (audit §1-4). `getItemKey` keys by path,
 * not by index, because the backend's parallel walk makes file arrival order non-deterministic
 * (d-50 S1a contract §3).
 */
export const SearchResultsList: FC<SearchResultsListProps> = ({ results, onOpenMatch, selection, className }) => {
    const parentRef = useRef<HTMLDivElement>(null)

    const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(new Set())

    const rows = buildSearchResultRows(results, collapsedPaths)

    const rowVirtualizer = useVirtualizer({
        count: rows.length,
        getScrollElement: () => parentRef.current,
        estimateSize: (index) => estimateSearchResultRowHeight(rows[index]),
        overscan: SEARCH_RESULT_OVERSCAN,
        getItemKey: (index) => rows[index].id,
    })

    const { t } = useTranslation()

    return (
        <div className={cn('relative min-h-0', className)}>
            <div ref={parentRef} className='scrollbar-hidden h-full w-full overflow-x-hidden overflow-y-auto'>
                <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
                    {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                        const row = rows[virtualRow.index]

                        return (
                            <div
                                key={virtualRow.key}
                                data-index={virtualRow.index}
                                ref={rowVirtualizer.measureElement}
                                style={{ position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${virtualRow.start}px)` }}>
                                {row.kind === 'group' ? (
                                    <FileGroupHeader
                                        path={row.path}
                                        count={row.matchCount}
                                        expanded={!row.collapsed}
                                        onToggle={() => setCollapsedPaths((current) => toggleInSet(current, row.path))}
                                        selected={selection ? !selection.excludedPaths.has(row.path) : undefined}
                                        onToggleSelect={selection ? () => selection.onToggleSelect(row.path) : undefined}
                                        selectAriaLabel={t('search.selectFile', { path: row.path })}
                                    />
                                ) : (
                                    <SearchMatchRow match={row.match} onClick={() => onOpenMatch(row.path, row.match.line, row.match.column)} />
                                )}
                            </div>
                        )
                    })}
                </div>
            </div>
            <OverlayScrollbar viewportRef={parentRef} orientation='vertical' />
        </div>
    )
}
