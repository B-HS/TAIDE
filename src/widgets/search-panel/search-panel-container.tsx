import type { FC } from 'react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectId } from '@shared/api/bindings'
import { DEFAULT_SEARCH_OPTIONS } from '@entities/search/search.type'
import { useRecentSearches } from '@entities/search/search-history'
import { useReplaceSearch } from '@entities/search/search.query'
import { useSearchRun } from '@entities/search/use-search-run'
import { requestReveal } from '@entities/editor/reveal-registry'
import { useOpenTab } from '@entities/layout/layout.query'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import type { ReplaceAllInput } from '@features/search/search-panel'
import { SearchPanel } from '@features/search/search-panel'

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
    const [caseSensitive, setCaseSensitive] = useState(DEFAULT_SEARCH_OPTIONS.caseSensitive)
    const [wholeWord, setWholeWord] = useState(DEFAULT_SEARCH_OPTIONS.wholeWord)
    const [regex, setRegex] = useState(DEFAULT_SEARCH_OPTIONS.regex)
    const [respectGitignore, setRespectGitignore] = useState(DEFAULT_SEARCH_OPTIONS.respectGitignore)
    const [excludeGlob, setExcludeGlob] = useState('')
    const [seededNonce, setSeededNonce] = useState<number | null>(null)

    const recentSearches = useRecentSearches()
    const { mutate: openTab } = useOpenTab(projectId)
    const { mutate: replaceAll, isPending: isReplacing } = useReplaceSearch()
    const sessionId = `search-panel-${useId()}`
    const { results, totalMatches, isSearching, run } = useSearchRun(projectId, sessionId)

    const scopePath = includeGlob ? includeGlob.replace(SCOPE_GLOB_SUFFIX, '') : null

    if (openNonce !== seededNonce) {
        setSeededNonce(openNonce)
        if (seedText) setQuery(seedText)
    }

    const buildQuery = () => ({
        text: query,
        caseSensitive,
        wholeWord,
        regex,
        includeGlob,
        excludeGlob: excludeGlob.trim() || null,
        respectGitignore,
    })

    const handleSubmit = () => {
        if (!query.trim()) return
        run(buildQuery())
    }

    const handleReplaceAll = (input: ReplaceAllInput) =>
        replaceAll(
            { projectId, query: buildQuery(), replacement: input.replacement, paths: input.paths.length > 0 ? input.paths : null },
            {
                onSuccess: (result) => {
                    toast.success(t('search.replaceDone', { files: result.changedFiles, matches: result.replacedMatches }))
                    handleSubmit()
                },
                onError: (error) => toast.error(describeIpcError(error)),
            },
        )

    const handleOpenMatch = (path: string, line: number, column: number) => {
        requestReveal(path, line, column)
        onOpenMatch(path)
    }

    const handleOpenInEditor = () => {
        const searchQuery = buildQuery()
        openTab(
            {
                projectId,
                kind: { kind: 'searchEditor', query: searchQuery },
                title: searchQuery.text || t('searchEditor.title'),
                target: null,
                preview: false,
            },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
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
            respectGitignore={respectGitignore}
            onRespectGitignoreChange={setRespectGitignore}
            excludeGlob={excludeGlob}
            onExcludeGlobChange={setExcludeGlob}
            history={recentSearches}
            onSelectHistory={setQuery}
            onSubmit={handleSubmit}
            isSearching={isSearching}
            totalMatches={totalMatches}
            results={results}
            onOpenMatch={handleOpenMatch}
            onReplaceAll={handleReplaceAll}
            isReplacing={isReplacing}
            scopePath={scopePath}
            onClearScope={onClearScope}
            openReplace={openReplace}
            openNonce={openNonce}
            onOpenInEditor={handleOpenInEditor}
        />
    )
}
