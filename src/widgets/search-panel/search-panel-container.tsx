import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectId, SearchMatch } from '@shared/api/bindings'
import { cancelSearch, replaceSearch, runSearch } from '@entities/search/search.ipc'
import type { ReplaceAllInput, SearchResultGroup } from '@widgets/search-panel/search-panel'
import { SearchPanel } from '@widgets/search-panel/search-panel'

type SearchPanelContainerProps = {
    projectId: ProjectId
    onOpenMatch: (path: string) => void
    includeGlob: string | null
    onClearScope: () => void
    seedText: string | null
    openReplace: boolean
    openNonce: number
}

const SCOPE_GLOB_SUFFIX = /\/\*\*$/

const groupMatches = (matches: SearchMatch[]) => {
    const byPath = new Map<string, SearchResultGroup>()
    for (const match of matches) {
        const group = byPath.get(match.path) ?? { path: match.path, matches: [] }
        group.matches.push({ line: match.line, column: match.column, preview: match.preview, matchStart: match.matchStart, matchEnd: match.matchEnd })
        byPath.set(match.path, group)
    }
    return [...byPath.values()]
}

export const SearchPanelContainer: FC<SearchPanelContainerProps> = ({
    projectId,
    onOpenMatch,
    includeGlob,
    onClearScope,
    seedText,
    openReplace,
    openNonce,
}) => {
    const { t } = useTranslation()
    const [query, setQuery] = useState('')
    const [caseSensitive, setCaseSensitive] = useState(false)
    const [wholeWord, setWholeWord] = useState(false)
    const [regex, setRegex] = useState(false)
    const [results, setResults] = useState<SearchResultGroup[]>([])
    const [totalMatches, setTotalMatches] = useState(0)
    const [isSearching, setIsSearching] = useState(false)
    const [isReplacing, setIsReplacing] = useState(false)
    const [seededNonce, setSeededNonce] = useState(openNonce)

    const scopePath = includeGlob ? includeGlob.replace(SCOPE_GLOB_SUFFIX, '') : null

    if (openNonce !== seededNonce) {
        setSeededNonce(openNonce)
        if (seedText) setQuery(seedText)
    }

    const handleSubmit = () => {
        if (!query.trim()) return

        const collected: SearchMatch[] = []
        setResults([])
        setTotalMatches(0)
        setIsSearching(true)

        void cancelSearch(projectId).catch(() => undefined)

        void runSearch({
            projectId,
            query: { text: query, caseSensitive, wholeWord, regex, includeGlob, excludeGlob: null },
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

    const handleReplaceAll = (input: ReplaceAllInput) => {
        setIsReplacing(true)
        void replaceSearch({
            projectId,
            query: { text: query, caseSensitive, wholeWord, regex, includeGlob, excludeGlob: null },
            replacement: input.replacement,
            paths: input.paths.length > 0 ? input.paths : null,
        })
            .then((result) => {
                toast.success(t('search.replaceDone', { files: result.changedFiles, matches: result.replacedMatches }))
                handleSubmit()
            })
            .catch((error: Error) => toast.error(error.message))
            .finally(() => setIsReplacing(false))
    }

    return (
        <SearchPanel
            query={query}
            onQueryChange={setQuery}
            caseSensitive={caseSensitive}
            onCaseSensitiveChange={setCaseSensitive}
            wholeWord={wholeWord}
            onWholeWordChange={setWholeWord}
            regex={regex}
            onRegexChange={setRegex}
            onSubmit={handleSubmit}
            isSearching={isSearching}
            totalMatches={totalMatches}
            results={results}
            onOpenMatch={onOpenMatch}
            onReplaceAll={handleReplaceAll}
            isReplacing={isReplacing}
            scopePath={scopePath}
            onClearScope={onClearScope}
            openReplace={openReplace}
            openNonce={openNonce}
        />
    )
}
