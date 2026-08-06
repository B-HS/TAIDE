import type { FC, KeyboardEvent } from 'react'
import { useState } from 'react'
import { CaseSensitive, Loader2, Search, WholeWord } from 'lucide-react'
import { SearchFileGroupHeader } from '@features/search/search-file-group-header'
import type { SearchMatchRowData } from '@features/search/search-match-row'
import { SearchMatchRow } from '@features/search/search-match-row'
import { cn } from '@shared/lib/cn'

export type SearchResultGroup = {
    path: string
    matches: SearchMatchRowData[]
}

type SearchPanelProps = {
    query: string
    onQueryChange: (value: string) => void
    caseSensitive: boolean
    onCaseSensitiveChange: (value: boolean) => void
    wholeWord: boolean
    onWholeWordChange: (value: boolean) => void
    onSubmit: () => void
    isSearching: boolean
    totalMatches: number
    results: SearchResultGroup[]
    onOpenMatch: (path: string) => void
}

const toggleInSet = (set: Set<string>, value: string) => {
    const next = new Set(set)
    if (next.has(value)) {
        next.delete(value)
    } else {
        next.add(value)
    }
    return next
}

export const SearchPanel: FC<SearchPanelProps> = ({
    query,
    onQueryChange,
    caseSensitive,
    onCaseSensitiveChange,
    wholeWord,
    onWholeWordChange,
    onSubmit,
    isSearching,
    totalMatches,
    results,
    onOpenMatch,
}) => {
    const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(new Set())

    const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key !== 'Enter') return
        onSubmit()
    }

    const hasQuery = query.trim().length > 0
    const hasResults = results.length > 0

    return (
        <div className='bg-panel-background flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border flex shrink-0 flex-col gap-1.5 border-b px-2 py-1.5'>
                <div className='bg-panel-input-background border-panel-input-border focus-within:border-app-focus-border flex items-center rounded-sm border'>
                    <input
                        value={query}
                        onChange={(event) => onQueryChange(event.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder='검색어 입력'
                        className='min-w-0 flex-1 bg-transparent px-2 py-1.5 text-xs outline-none'
                    />
                    <button
                        type='button'
                        aria-label='대소문자 구분'
                        aria-pressed={caseSensitive}
                        onClick={() => onCaseSensitiveChange(!caseSensitive)}
                        className={cn(
                            'flex size-6 shrink-0 items-center justify-center rounded-sm',
                            caseSensitive
                                ? 'bg-explorer-item-selected text-app-foreground'
                                : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover',
                        )}>
                        <CaseSensitive className='size-3.5' />
                    </button>
                    <button
                        type='button'
                        aria-label='단어 단위 검색'
                        aria-pressed={wholeWord}
                        onClick={() => onWholeWordChange(!wholeWord)}
                        className={cn(
                            'mr-1 flex size-6 shrink-0 items-center justify-center rounded-sm',
                            wholeWord
                                ? 'bg-explorer-item-selected text-app-foreground'
                                : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover',
                        )}>
                        <WholeWord className='size-3.5' />
                    </button>
                </div>
                {isSearching && (
                    <div className='text-app-sidebar-icon-default flex items-center gap-1.5 text-xs'>
                        <Loader2 className='size-3 animate-spin' />
                        검색 중…
                    </div>
                )}
                {!isSearching && hasQuery && (
                    <div className='text-app-sidebar-icon-default text-xs'>
                        {totalMatches > 0 ? `${totalMatches}개 결과, 파일 ${results.length}개` : '결과 없음'}
                    </div>
                )}
            </div>

            <div className='min-h-0 flex-1 overflow-y-auto'>
                {!hasQuery && (
                    <div className='text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-2 px-4 text-center text-xs'>
                        <Search className='size-5 opacity-60' />
                        검색어를 입력하고 Enter 를 누르세요
                    </div>
                )}
                {hasQuery && !isSearching && !hasResults && (
                    <div className='text-app-sidebar-icon-default flex h-full w-full items-center justify-center px-4 text-center text-xs'>
                        일치하는 결과가 없습니다
                    </div>
                )}
                {hasResults &&
                    results.map((group) => {
                        const collapsed = collapsedPaths.has(group.path)
                        return (
                            <div key={group.path}>
                                <SearchFileGroupHeader
                                    path={group.path}
                                    matchCount={group.matches.length}
                                    expanded={!collapsed}
                                    onToggle={() => setCollapsedPaths((current) => toggleInSet(current, group.path))}
                                />
                                {!collapsed &&
                                    group.matches.map((match) => (
                                        <SearchMatchRow
                                            key={`${group.path}:${match.line}:${match.column}`}
                                            match={match}
                                            onClick={() => onOpenMatch(group.path)}
                                        />
                                    ))}
                            </div>
                        )
                    })}
            </div>
        </div>
    )
}
