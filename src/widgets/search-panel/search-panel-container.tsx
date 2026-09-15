import type { FC } from 'react'
import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { ProjectId, ProjectRef } from '@shared/api/bindings'
import { DEFAULT_SEARCH_OPTIONS } from '@entities/search/search.type'
import { isSameSearchQuery } from '@entities/search/search-query'
import { buildReplaceSkipReport, REPLACE_SKIP_REASON_MESSAGE_KEY } from '@entities/search/replace-skip-report'
import { useRecentSearches } from '@entities/search/search-history'
import { useReplaceSearch } from '@entities/search/search.query'
import { useSearchRun } from '@entities/search/use-search-run'
import { requestReveal } from '@entities/editor/reveal-registry'
import { notifyNative } from '@entities/notification/notify'
import { useOpenTab } from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { resolveProjectNotificationTitle } from '@shared/lib/notification-text'
import { PERF_MARK, PERF_MEASURE, perfMark, perfMeasure } from '@shared/lib/perf-mark'
import { DEFAULT_SEARCH_ON_TYPE, DEFAULT_SEARCH_ON_TYPE_DEBOUNCE_MS } from '@shared/constants/search'
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
    const liveRunTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const liveRunRef = useRef<() => void>(() => undefined)
    const isLiveRunRef = useRef(false)

    const { t } = useTranslation()
    const [query, setQuery] = useState('')
    const [caseSensitive, setCaseSensitive] = useState(DEFAULT_SEARCH_OPTIONS.caseSensitive)
    const [wholeWord, setWholeWord] = useState(DEFAULT_SEARCH_OPTIONS.wholeWord)
    const [regex, setRegex] = useState(DEFAULT_SEARCH_OPTIONS.regex)
    const [respectGitignore, setRespectGitignore] = useState(DEFAULT_SEARCH_OPTIONS.respectGitignore)
    const [excludeGlob, setExcludeGlob] = useState('')
    const [seededNonce, setSeededNonce] = useState<number | null>(null)

    const recentSearches = useRecentSearches()
    const { data: settings } = useQuery(settingsQueryOptions())
    const queryClient = useQueryClient()
    const { mutate: openTab } = useOpenTab(projectId)
    const { mutate: replaceAll, isPending: isReplacing } = useReplaceSearch()
    const sessionId = `search-panel-${useId()}`
    const { results, totalMatches, status, ranQuery, isTruncated, run } = useSearchRun(projectId, sessionId)

    const scopePath = includeGlob ? includeGlob.replace(SCOPE_GLOB_SUFFIX, '') : null
    const isLiveSearchEnabled = settings?.searchOnType ?? DEFAULT_SEARCH_ON_TYPE
    const liveSearchDebounceMs = settings?.searchOnTypeDebounceMs ?? DEFAULT_SEARCH_ON_TYPE_DEBOUNCE_MS

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

    const queryMatchesResults = ranQuery !== null && isSameSearchQuery(buildQuery(), ranQuery)

    const cancelPendingLiveRun = () => {
        clearTimeout(liveRunTimerRef.current)
        liveRunTimerRef.current = undefined
    }

    /**
     * Opens the search-render span (the global-search metric in
     * `docs/quality-assurance/2026-09-04-perf-baseline.md`) on the submit itself, so the
     * measurement covers the backend walk *and* the first flush's render — what the user waits for
     * — rather than the render alone. The re-run that follows a replace deliberately carries no
     * mark: its results are a refresh of a list already on screen. Neither does a search-as-you-type
     * run, which is why the pending one is dropped here: Enter must not be timed against a span a
     * keystroke opened, and a debounced run left armed would fire a duplicate right after this one.
     */
    const handleSubmit = () => {
        if (!query.trim()) return
        cancelPendingLiveRun()
        isLiveRunRef.current = false
        perfMark(PERF_MARK.SEARCH_RUN_REQUESTED)
        run(buildQuery())
    }

    /**
     * Replaces with the query the displayed results were produced by, never with whatever the
     * inputs hold at click time — editing the term or flipping a toggle after searching used to
     * rewrite matches the user had never been shown, irreversibly (audit §4-B A5). The button is
     * disabled while the two disagree, and the re-run afterwards uses that same snapshot so the
     * refreshed list still corresponds to what was replaced.
     */
    const handleReplaceAll = (input: ReplaceAllInput) => {
        if (!ranQuery) return
        const replacedQuery = ranQuery

        replaceAll(
            { projectId, query: replacedQuery, replacement: input.replacement, paths: input.paths.length > 0 ? input.paths : null },
            {
                onSuccess: (result) => {
                    const replaceSummary = t('search.replaceDone', { files: result.changedFiles, matches: result.replacedMatches })
                    toast.success(replaceSummary)
                    void notifyNative({
                        category: 'searchReplace',
                        title: resolveProjectNotificationTitle({
                            projects: queryClient.getQueryData<ProjectRef[]>(QUERY_KEY.PROJECT.LIST),
                            projectId,
                            fallbackTitle: t('notification.searchReplaceDone'),
                        }),
                        body: replaceSummary,
                    })
                    const report = buildReplaceSkipReport(result, (file) => `${file.path} — ${t(REPLACE_SKIP_REASON_MESSAGE_KEY[file.reason])}`)
                    if (report)
                        toast.warning(t('search.replaceSkipped', { count: report.total }), {
                            description: (
                                <div className='flex flex-col gap-0.5'>
                                    {report.lines.map((line) => (
                                        <span key={line} className='truncate'>
                                            {line}
                                        </span>
                                    ))}
                                    {report.remaining > 0 && <span>{t('search.replaceSkippedMore', { count: report.remaining })}</span>}
                                </div>
                            ),
                        })
                    run(replacedQuery, { recordHistory: false })
                },
                onError: (error) => toast.error(describeIpcError(error)),
            },
        )
    }

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

    /**
     * Hands the debounce below the current render's run. `run` is rebuilt on every render — the
     * renders the run itself causes included — so an effect that listed it as a dependency would
     * re-arm the timer after each of those and search in a loop. The timer depends on the query and
     * its options alone, and reads the live closure from here when it fires.
     */
    useEffect(() => {
        liveRunRef.current = () => {
            isLiveRunRef.current = true
            run(buildQuery(), { recordHistory: false, live: true })
        }
    })

    /**
     * Search as you type (`settings.searchOnType`): trailing-only, so a run fires once typing has
     * paused rather than on the first keystroke of a term nobody has finished. Nothing cancels the
     * superseded backend run — `search_run`'s own `begin_search` supersedes it, and an explicit
     * `searchCancel` raced that (see `use-search-run.ts`). WKWebView delivers no composition events
     * (`shared/lib/ime-composition.ts`), so a Hangul/Kana term is searched at whatever composition
     * state the pause caught; the debounce is the whole mitigation, and Enter stays authoritative.
     */
    useEffect(() => {
        if (!isLiveSearchEnabled || !query.trim()) return
        liveRunTimerRef.current = setTimeout(() => liveRunRef.current(), liveSearchDebounceMs)
        return () => clearTimeout(liveRunTimerRef.current)
    }, [isLiveSearchEnabled, liveSearchDebounceMs, query, caseSensitive, wholeWord, regex, respectGitignore, excludeGlob, includeGlob])

    /**
     * Closes the span at the first batch that actually produced rows — `useSearchRun` clears
     * `results` to an empty array the moment a run starts, and an empty commit paints nothing worth
     * timing. A run that finds no match at all leaves its start mark unconsumed until the next
     * submit overwrites it, which is what keeps "no results" out of the sample. A search-as-you-type
     * run is excluded on both ends: it opens no mark, and it must not *close* one either, or the
     * keystroke that follows a fruitless submit would report that submit's age as a search duration.
     */
    useEffect(() => {
        if (results.length === 0 || isLiveRunRef.current) return
        perfMeasure(PERF_MEASURE.SEARCH_RESULTS, PERF_MARK.SEARCH_RUN_REQUESTED)
    }, [results])

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
            isLiveSearch={isLiveSearchEnabled}
            status={status}
            totalMatches={totalMatches}
            isTruncated={isTruncated}
            results={results}
            onOpenMatch={handleOpenMatch}
            onReplaceAll={handleReplaceAll}
            isReplacing={isReplacing}
            queryMatchesResults={queryMatchesResults}
            scopePath={scopePath}
            onClearScope={onClearScope}
            openReplace={openReplace}
            openNonce={openNonce}
            onOpenInEditor={handleOpenInEditor}
        />
    )
}
