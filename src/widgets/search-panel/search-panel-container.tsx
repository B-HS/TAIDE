import type { FC } from 'react'
import { useState } from 'react'
import { toast } from 'sonner'
import type { ProjectId, SearchMatch } from '@shared/api/bindings'
import { cancelSearch, runSearch } from '@entities/search/search.ipc'
import type { SearchResultGroup } from '@widgets/search-panel/search-panel'
import { SearchPanel } from '@widgets/search-panel/search-panel'

type SearchPanelContainerProps = {
    projectId: ProjectId
    onOpenMatch: (path: string) => void
}

const groupMatches = (matches: SearchMatch[]) => {
    const byPath = new Map<string, SearchResultGroup>()
    for (const match of matches) {
        const group = byPath.get(match.path) ?? { path: match.path, matches: [] }
        group.matches.push({ line: match.line, column: match.column, preview: match.preview, matchStart: match.matchStart, matchEnd: match.matchEnd })
        byPath.set(match.path, group)
    }
    return [...byPath.values()]
}

export const SearchPanelContainer: FC<SearchPanelContainerProps> = ({ projectId, onOpenMatch }) => {
    const [query, setQuery] = useState('')
    const [caseSensitive, setCaseSensitive] = useState(false)
    const [wholeWord, setWholeWord] = useState(false)
    const [results, setResults] = useState<SearchResultGroup[]>([])
    const [totalMatches, setTotalMatches] = useState(0)
    const [isSearching, setIsSearching] = useState(false)

    const handleSubmit = () => {
        if (!query.trim()) return

        const collected: SearchMatch[] = []
        setResults([])
        setTotalMatches(0)
        setIsSearching(true)

        void cancelSearch(projectId).catch(() => undefined)

        void runSearch({
            projectId,
            query: { text: query, caseSensitive, wholeWord, regex: false, includeGlob: null, excludeGlob: null },
            onMatch: (match) => {
                collected.push(match)
                setResults(groupMatches(collected))
                setTotalMatches(collected.length)
            },
        })
            .then((total) => setTotalMatches(total))
            .catch((error: Error) => toast.error(error.message))
            .finally(() => setIsSearching(false))
    }

    return (
        <SearchPanel
            query={query}
            onQueryChange={setQuery}
            caseSensitive={caseSensitive}
            onCaseSensitiveChange={setCaseSensitive}
            wholeWord={wholeWord}
            onWholeWordChange={setWholeWord}
            onSubmit={handleSubmit}
            isSearching={isSearching}
            totalMatches={totalMatches}
            results={results}
            onOpenMatch={onOpenMatch}
        />
    )
}
