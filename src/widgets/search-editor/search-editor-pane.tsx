import type { FC, KeyboardEvent } from 'react'
import { useEffect, useRef, useState } from 'react'
import { Loader2, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId, SearchQuery, TabId } from '@shared/api/bindings'
import { DEFAULT_SEARCH_OPTIONS } from '@entities/search/search.type'
import type { SearchEditorFormState } from '@entities/search/search-editor-memory'
import { readSearchEditorMemory, writeSearchEditorMemory } from '@entities/search/search-editor-memory'
import type { SearchRunSnapshot } from '@entities/search/search-run-state'
import { resolveSearchResultsView } from '@entities/search/search-run-state'
import { useRecentSearches } from '@entities/search/search-history'
import { useSearchRun } from '@entities/search/use-search-run'
import { requestReveal } from '@entities/editor/reveal-registry'
import { layoutQueryOptions, useOpenFileTab } from '@entities/layout/layout.query'
import { SearchExcludeGlobInput } from '@features/search/search-exclude-glob-input'
import { SearchHistoryDropdown } from '@features/search/search-history-dropdown'
import { SearchOptionToggles } from '@features/search/search-option-toggles'
import { SearchResultsList } from '@features/search/search-results-list'
import {
    clampContextLines,
    SEARCH_EDITOR_MAX_CONTEXT_LINES,
    SEARCH_EDITOR_MIN_CONTEXT_LINES,
} from '@widgets/search-editor/search-editor-context-lines'
import { SEARCH_MATCH_LIMIT } from '@shared/constants/search'
import { isImeCompositionKeydown } from '@shared/lib/ime-composition'
import { currentWindowFocusedPane } from '@shared/lib/pane-tree'
import { ScrollContainer } from '@shared/scroll/scroll-container'

type SearchEditorPaneProps = {
    projectId: ProjectId
    tabId: TabId
    query: SearchQuery
}

/**
 * A Search Editor tab's pane. Only the *active* tab of a pane is mounted, so opening a match —
 * which targets this very pane — unmounts this component; `entities/search/search-editor-memory.ts`
 * is what carries the inputs and the result list across that gap, and is also what stops the
 * mount-time auto-run from firing a second time and overwriting restored results with the tab's
 * original query (audit §4-B B8).
 */
export const SearchEditorPane: FC<SearchEditorPaneProps> = ({ projectId, tabId, query }) => {
    const queryInputRef = useRef<HTMLInputElement>(null)
    const initialQueryRef = useRef(query)
    const didAutoRunRef = useRef(false)
    const latestRef = useRef<{ form: SearchEditorFormState; readSnapshot: () => SearchRunSnapshot } | null>(null)

    const [restored] = useState(() => readSearchEditorMemory(tabId, projectId))
    const restoredForm = restored?.form ?? null

    const [queryText, setQueryText] = useState(restoredForm?.queryText ?? query.text)
    const [caseSensitive, setCaseSensitive] = useState(restoredForm?.caseSensitive ?? query.caseSensitive ?? DEFAULT_SEARCH_OPTIONS.caseSensitive)
    const [wholeWord, setWholeWord] = useState(restoredForm?.wholeWord ?? query.wholeWord ?? DEFAULT_SEARCH_OPTIONS.wholeWord)
    const [regex, setRegex] = useState(restoredForm?.regex ?? query.regex ?? DEFAULT_SEARCH_OPTIONS.regex)
    const [respectGitignore, setRespectGitignore] = useState(
        restoredForm?.respectGitignore ?? query.respectGitignore ?? DEFAULT_SEARCH_OPTIONS.respectGitignore,
    )
    const [excludeGlob, setExcludeGlob] = useState(restoredForm?.excludeGlob ?? query.excludeGlob ?? DEFAULT_SEARCH_OPTIONS.excludeGlob ?? '')
    const [contextLines, setContextLines] = useState(
        clampContextLines(restoredForm?.contextLines ?? query.contextLines ?? DEFAULT_SEARCH_OPTIONS.contextLines),
    )

    const { t } = useTranslation()
    const recentSearches = useRecentSearches()
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const openFileTab = useOpenFileTab()
    const { results, totalMatches, status, isTruncated, run, readSnapshot } = useSearchRun(projectId, tabId, restored?.run)

    const hasResults = results.length > 0
    const view = resolveSearchResultsView({ status, hasResults })
    const includeGlob = query.includeGlob ?? null
    const form: SearchEditorFormState = { queryText, caseSensitive, wholeWord, regex, respectGitignore, excludeGlob, contextLines }

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
        if (isImeCompositionKeydown(event) || event.key !== 'Enter') return
        handleSubmit()
    }

    const handleOpenMatch = (path: string, line: number, column: number) => {
        requestReveal(path, line, column)
        openFileTab({ projectId, path, target: currentWindowFocusedPane(layout), preview: true })
    }

    useEffect(() => {
        latestRef.current = { form, readSnapshot }
    })

    useEffect(() => {
        if (didAutoRunRef.current || restored) return
        const initialQuery = initialQueryRef.current
        if (!initialQuery.text.trim()) return
        didAutoRunRef.current = true
        run(initialQuery, { recordHistory: false })
    }, [restored, run])

    useEffect(() => {
        queryInputRef.current?.focus()
    }, [])

    useEffect(
        () => () => {
            const latest = latestRef.current
            if (!latest) return
            writeSearchEditorMemory(tabId, { projectId, form: latest.form, run: latest.readSnapshot() })
        },
        [projectId, tabId],
    )

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
                {status === 'running' && (
                    <div className='text-app-sidebar-icon-default flex items-center gap-1.5 text-xs'>
                        <Loader2 className='size-3 animate-spin' />
                        {t('search.searching')}
                    </div>
                )}
                {status === 'completed' && (
                    <div className='text-app-sidebar-icon-default text-xs'>
                        {totalMatches > 0 ? t('search.matchCount', { count: totalMatches, files: results.length }) : t('searchEditor.noResults')}
                    </div>
                )}
                {status === 'completed' && isTruncated && (
                    <div className='text-panel-match-highlight text-xs'>{t('search.truncated', { limit: SEARCH_MATCH_LIMIT })}</div>
                )}
            </div>

            {view === 'results' ? (
                <SearchResultsList className='flex-1' results={results} onOpenMatch={handleOpenMatch} />
            ) : (
                <ScrollContainer className='min-h-0 flex-1'>
                    {view === 'hint' && (
                        <div className='text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-2 px-4 text-center text-xs'>
                            <Search className='size-5 opacity-60' />
                            {t('search.pressEnterHint')}
                        </div>
                    )}
                    {view === 'empty' && (
                        <div className='text-app-sidebar-icon-default flex h-full w-full items-center justify-center px-4 text-center text-xs'>
                            {t('searchEditor.noResults')}
                        </div>
                    )}
                    {view === 'failed' && (
                        <div className='text-app-sidebar-icon-default flex h-full w-full items-center justify-center px-4 text-center text-xs'>
                            {t('search.failed')}
                        </div>
                    )}
                </ScrollContainer>
            )}
        </div>
    )
}
