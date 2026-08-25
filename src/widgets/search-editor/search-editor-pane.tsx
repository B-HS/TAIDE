import type { FC, KeyboardEvent } from 'react'
import { useEffect, useRef, useState } from 'react'
import { Loader2, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId, SearchQuery, TabId } from '@shared/api/bindings'
import { DEFAULT_SEARCH_OPTIONS } from '@entities/search/search.type'
import { useRecentSearches } from '@entities/search/search-history'
import { useSearchRun } from '@entities/search/use-search-run'
import { requestReveal } from '@entities/editor/reveal-registry'
import { layoutQueryOptions, useOpenTab } from '@entities/layout/layout.query'
import { SearchExcludeGlobInput } from '@features/search/search-exclude-glob-input'
import { SearchHistoryDropdown } from '@features/search/search-history-dropdown'
import { SearchOptionToggles } from '@features/search/search-option-toggles'
import { SearchResultsList } from '@features/search/search-results-list'
import {
    clampContextLines,
    SEARCH_EDITOR_MAX_CONTEXT_LINES,
    SEARCH_EDITOR_MIN_CONTEXT_LINES,
} from '@widgets/search-editor/search-editor-context-lines'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { currentWindowFocusedPane } from '@shared/lib/pane-tree'
import { fileNameOf } from '@shared/lib/relative-path'
import { ScrollContainer } from '@shared/scroll/scroll-container'

type SearchEditorPaneProps = {
    projectId: ProjectId
    tabId: TabId
    query: SearchQuery
}

export const SearchEditorPane: FC<SearchEditorPaneProps> = ({ projectId, tabId, query }) => {
    const { t } = useTranslation()
    const queryInputRef = useRef<HTMLInputElement>(null)
    const initialQueryRef = useRef(query)
    const didAutoRunRef = useRef(false)

    const [queryText, setQueryText] = useState(query.text)
    const [caseSensitive, setCaseSensitive] = useState(query.caseSensitive ?? DEFAULT_SEARCH_OPTIONS.caseSensitive)
    const [wholeWord, setWholeWord] = useState(query.wholeWord ?? DEFAULT_SEARCH_OPTIONS.wholeWord)
    const [regex, setRegex] = useState(query.regex ?? DEFAULT_SEARCH_OPTIONS.regex)
    const [respectGitignore, setRespectGitignore] = useState(query.respectGitignore ?? DEFAULT_SEARCH_OPTIONS.respectGitignore)
    const [excludeGlob, setExcludeGlob] = useState(query.excludeGlob ?? DEFAULT_SEARCH_OPTIONS.excludeGlob ?? '')
    const [contextLines, setContextLines] = useState(clampContextLines(query.contextLines ?? DEFAULT_SEARCH_OPTIONS.contextLines))

    const recentSearches = useRecentSearches()
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: openTab } = useOpenTab(projectId)
    const { results, totalMatches, isSearching, run } = useSearchRun(projectId, tabId)

    const hasQuery = queryText.trim().length > 0
    const hasResults = results.length > 0
    const includeGlob = query.includeGlob ?? null

    const handleSubmit = () => {
        if (!queryText.trim()) return
        run({
            text: queryText,
            caseSensitive,
            wholeWord,
            regex,
            includeGlob,
            excludeGlob: excludeGlob.trim() || null,
            contextLines,
            respectGitignore,
        })
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key !== 'Enter') return
        handleSubmit()
    }

    const handleOpenMatch = (path: string, line: number, column: number) => {
        requestReveal(path, line, column)
        openTab(
            { projectId, kind: { kind: 'file', path }, title: fileNameOf(path), target: currentWindowFocusedPane(layout), preview: true },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    useEffect(() => {
        if (didAutoRunRef.current) return
        const initialQuery = initialQueryRef.current
        if (!initialQuery.text.trim()) return
        didAutoRunRef.current = true
        run(initialQuery, { recordHistory: false })
    }, [run])

    useEffect(() => {
        queryInputRef.current?.focus()
    }, [])

    return (
        <div className='bg-editor-background flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border flex shrink-0 flex-col gap-1.5 border-b px-3 py-2'>
                <div className='bg-panel-input-background border-panel-input-border focus-within:border-app-focus-border flex items-center rounded-sm border'>
                    <input
                        ref={queryInputRef}
                        value={queryText}
                        onChange={(event) => setQueryText(event.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder={t('search.placeholder')}
                        className='min-w-0 flex-1 bg-transparent px-2 py-1.5 text-xs outline-none'
                    />
                    <SearchHistoryDropdown history={recentSearches} onSelect={setQueryText} />
                    <SearchOptionToggles
                        caseSensitive={caseSensitive}
                        onCaseSensitiveChange={setCaseSensitive}
                        wholeWord={wholeWord}
                        onWholeWordChange={setWholeWord}
                        regex={regex}
                        onRegexChange={setRegex}
                        respectGitignore={respectGitignore}
                        onRespectGitignoreChange={setRespectGitignore}
                    />
                </div>
                <div className='flex flex-wrap items-center gap-3'>
                    <SearchExcludeGlobInput value={excludeGlob} onChange={setExcludeGlob} />
                    <label className='text-app-sidebar-icon-default flex shrink-0 items-center gap-1.5 text-xs'>
                        {t('searchEditor.contextLinesLabel')}
                        <input
                            type='number'
                            min={SEARCH_EDITOR_MIN_CONTEXT_LINES}
                            max={SEARCH_EDITOR_MAX_CONTEXT_LINES}
                            value={contextLines}
                            onChange={(event) => setContextLines(clampContextLines(Number(event.target.value)))}
                            className='bg-panel-input-background border-panel-input-border focus:border-app-focus-border w-14 rounded-sm border px-1.5 py-1 text-xs outline-none'
                        />
                    </label>
                </div>
                {isSearching && (
                    <div className='text-app-sidebar-icon-default flex items-center gap-1.5 text-xs'>
                        <Loader2 className='size-3 animate-spin' />
                        {t('search.searching')}
                    </div>
                )}
                {!isSearching && hasQuery && (
                    <div className='text-app-sidebar-icon-default text-xs'>
                        {totalMatches > 0 ? t('search.matchCount', { count: totalMatches, files: results.length }) : t('searchEditor.noResults')}
                    </div>
                )}
            </div>

            <ScrollContainer className='min-h-0 flex-1'>
                {!hasQuery && (
                    <div className='text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-2 px-4 text-center text-xs'>
                        <Search className='size-5 opacity-60' />
                        {t('search.pressEnterHint')}
                    </div>
                )}
                {hasQuery && !isSearching && !hasResults && (
                    <div className='text-app-sidebar-icon-default flex h-full w-full items-center justify-center px-4 text-center text-xs'>
                        {t('searchEditor.noResults')}
                    </div>
                )}
                {hasResults && <SearchResultsList results={results} onOpenMatch={handleOpenMatch} />}
            </ScrollContainer>
        </div>
    )
}
