import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { SearchResultGroup } from '@entities/search/search-result'
import { SearchMatchRow } from '@features/search/search-match-row'
import { toggleInSet } from '@shared/lib/set'
import { FileGroupHeader } from '@shared/ui/file-group-header'

type SearchResultsListSelection = {
    excludedPaths: Set<string>
    onToggleSelect: (path: string) => void
}

type SearchResultsListProps = {
    results: SearchResultGroup[]
    onOpenMatch: (path: string, line: number, column: number) => void
    selection?: SearchResultsListSelection
}

export const SearchResultsList: FC<SearchResultsListProps> = ({ results, onOpenMatch, selection }) => {
    const { t } = useTranslation()
    const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(new Set())

    return (
        <>
            {results.map((group) => {
                const collapsed = collapsedPaths.has(group.path)
                return (
                    <div key={group.path}>
                        <FileGroupHeader
                            path={group.path}
                            count={group.matches.length}
                            expanded={!collapsed}
                            onToggle={() => setCollapsedPaths((current) => toggleInSet(current, group.path))}
                            selected={selection ? !selection.excludedPaths.has(group.path) : undefined}
                            onToggleSelect={selection ? () => selection.onToggleSelect(group.path) : undefined}
                            selectAriaLabel={t('search.selectFile', { path: group.path })}
                        />
                        {!collapsed &&
                            group.matches.map((match) => (
                                <SearchMatchRow
                                    key={`${group.path}:${match.line}:${match.column}`}
                                    match={match}
                                    onClick={() => onOpenMatch(group.path, match.line, match.column)}
                                />
                            ))}
                    </div>
                )
            })}
        </>
    )
}
