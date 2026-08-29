import type { FC, KeyboardEvent } from 'react'
import { useEffect, useRef, useState } from 'react'
import { ChevronRight, FileSearch2, Loader2, ReplaceAll, Search, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { SearchResultGroup } from '@entities/search/search-result'
import type { SearchRunStatus } from '@entities/search/search-run-state'
import { resolveSearchResultsView } from '@entities/search/search-run-state'
import { SEARCH_MATCH_LIMIT } from '@shared/constants/search'
import { SearchExcludeGlobInput } from '@features/search/search-exclude-glob-input'
import { SearchHistoryDropdown } from '@features/search/search-history-dropdown'
import { SearchOptionToggles } from '@features/search/search-option-toggles'
import { SearchResultsList } from '@features/search/search-results-list'
import { cn } from '@shared/lib/cn'
import { isImeCompositionKeydown } from '@shared/lib/ime-composition'
import { toggleInSet } from '@shared/lib/set'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'
import { IconButton } from '@shared/ui/icon-button'
import { ScrollContainer } from '@shared/scroll/scroll-container'

export type ReplaceAllInput = {
    replacement: string
    paths: string[]
}

type SearchPanelProps = {
    query: string
    onQueryChange: (value: string) => void
    caseSensitive: boolean
    onCaseSensitiveChange: (value: boolean) => void
    wholeWord: boolean
    onWholeWordChange: (value: boolean) => void
    regex: boolean
    onRegexChange: (value: boolean) => void
    respectGitignore: boolean
    onRespectGitignoreChange: (value: boolean) => void
    excludeGlob: string
    onExcludeGlobChange: (value: string) => void
    history: string[]
    onSelectHistory: (term: string) => void
    onSubmit: () => void
    status: SearchRunStatus
    totalMatches: number
    isTruncated: boolean
    results: SearchResultGroup[]
    onOpenMatch: (path: string, line: number, column: number) => void
    onReplaceAll: (input: ReplaceAllInput) => void
    isReplacing: boolean
    /**
     * Whether the inputs still describe the query the displayed results came from. Replace All
     * rewrites files it was never shown, so it stays disabled while they disagree — audit §4-B A5.
     */
    queryMatchesResults: boolean
    scopePath: string | null
    onClearScope: () => void
    openReplace: boolean
    openNonce: number
    onOpenInEditor: () => void
}

export const SearchPanel: FC<SearchPanelProps> = ({
    query,
    onQueryChange,
    caseSensitive,
    onCaseSensitiveChange,
    wholeWord,
    onWholeWordChange,
    regex,
    onRegexChange,
    respectGitignore,
    onRespectGitignoreChange,
    excludeGlob,
    onExcludeGlobChange,
    history,
    onSelectHistory,
    onSubmit,
    status,
    totalMatches,
    isTruncated,
    results,
    onOpenMatch,
    onReplaceAll,
    isReplacing,
    queryMatchesResults,
    scopePath,
    onClearScope,
    openReplace,
    openNonce,
    onOpenInEditor,
}) => {
    const { t } = useTranslation()
    const queryInputRef = useRef<HTMLInputElement>(null)
    const [replaceOpen, setReplaceOpen] = useState(false)
    const [replaceText, setReplaceText] = useState('')
    const [excludedPaths, setExcludedPaths] = useState<Set<string>>(new Set())
    const [confirmOpen, setConfirmOpen] = useState(false)
    const [appliedOpenNonce, setAppliedOpenNonce] = useState<number | null>(null)

    if (openNonce !== appliedOpenNonce) {
        setAppliedOpenNonce(openNonce)
        if (openReplace) setReplaceOpen(true)
    }

    const hasResults = results.length > 0
    const view = resolveSearchResultsView({ status, hasResults })
    const selectedGroups = results.filter((group) => !excludedPaths.has(group.path))
    const selectedMatchCount = selectedGroups.reduce((sum, group) => sum + group.matches.length, 0)
    const canReplaceAll = replaceOpen && selectedGroups.length > 0 && !isReplacing && queryMatchesResults

    const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        if (isImeCompositionKeydown(event) || event.key !== 'Enter') return
        onSubmit()
    }

    const handleConfirmReplace = () => {
        onReplaceAll({ replacement: replaceText, paths: selectedGroups.map((group) => group.path) })
        setConfirmOpen(false)
    }

    useEffect(() => {
        queryInputRef.current?.focus()
        queryInputRef.current?.select()
    }, [openNonce])

    return (
        <div className='bg-panel-background flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border flex shrink-0 flex-col gap-1.5 border-b px-2 py-1.5'>
                <div className='flex items-start gap-1'>
                    <IconButton
                        label={t('search.replaceToggle')}
                        icon={<ChevronRight className={cn('size-3.5 transition-transform', replaceOpen && 'rotate-90')} />}
                        aria-pressed={replaceOpen}
                        onClick={() => setReplaceOpen(!replaceOpen)}
                        side='bottom'
                        containerClassName='mt-0.5'
                        className='text-app-sidebar-icon-default hover:bg-explorer-item-hover flex size-6 shrink-0 items-center justify-center rounded-sm'
                    />
                    <div className='flex min-w-0 flex-1 flex-col gap-1'>
                        <div className='bg-panel-input-background border-panel-input-border focus-within:border-app-focus-border flex items-center rounded-sm border'>
                            <input
                                ref={queryInputRef}
                                value={query}
                                onChange={(event) => onQueryChange(event.target.value)}
                                onKeyDown={handleKeyDown}
                                placeholder={t('search.placeholder')}
                                className='min-w-0 flex-1 bg-transparent px-2 py-1.5 text-xs outline-none'
                            />
                            <SearchHistoryDropdown history={history} onSelect={onSelectHistory} />
                            <SearchOptionToggles
                                caseSensitive={caseSensitive}
                                onCaseSensitiveChange={onCaseSensitiveChange}
                                wholeWord={wholeWord}
                                onWholeWordChange={onWholeWordChange}
                                regex={regex}
                                onRegexChange={onRegexChange}
                                respectGitignore={respectGitignore}
                                onRespectGitignoreChange={onRespectGitignoreChange}
                            />
                        </div>
                        {replaceOpen && (
                            <div className='bg-panel-input-background border-panel-input-border focus-within:border-app-focus-border flex items-center rounded-sm border'>
                                <input
                                    value={replaceText}
                                    onChange={(event) => setReplaceText(event.target.value)}
                                    placeholder={t('search.replacePlaceholder')}
                                    className='min-w-0 flex-1 bg-transparent px-2 py-1.5 text-xs outline-none'
                                />
                                <IconButton
                                    label={t('search.replaceAll')}
                                    icon={<ReplaceAll className='size-3.5' />}
                                    disabled={!canReplaceAll}
                                    onClick={() => setConfirmOpen(true)}
                                    side='bottom'
                                    containerClassName='mr-1'
                                    className='text-app-sidebar-icon-default hover:bg-explorer-item-hover flex size-6 shrink-0 items-center justify-center rounded-sm disabled:pointer-events-none disabled:opacity-40'
                                />
                            </div>
                        )}
                        {replaceOpen && hasResults && !queryMatchesResults && (
                            <div className='text-app-sidebar-icon-default text-xs'>{t('search.replaceStaleHint')}</div>
                        )}
                        <SearchExcludeGlobInput value={excludeGlob} onChange={onExcludeGlobChange} />
                    </div>
                    <IconButton
                        label={t('searchEditor.title')}
                        icon={<FileSearch2 className='size-3.5' />}
                        onClick={onOpenInEditor}
                        side='bottom'
                        containerClassName='mt-0.5'
                        className='text-app-sidebar-icon-default hover:bg-explorer-item-hover flex size-6 shrink-0 items-center justify-center rounded-sm'
                    />
                </div>
                {scopePath && (
                    <div className='bg-explorer-item-selected text-app-foreground flex w-fit max-w-full items-center gap-1 rounded-sm px-1.5 py-0.5 text-xs'>
                        <span className='truncate'>{t('explorer.searchScopeLabel', { path: scopePath })}</span>
                        <IconButton
                            label={t('search.clearScope')}
                            icon={<X className='size-3' />}
                            onClick={onClearScope}
                            side='bottom'
                            className='text-app-sidebar-icon-default hover:text-app-foreground shrink-0'
                        />
                    </div>
                )}
                {status === 'running' && (
                    <div className='text-app-sidebar-icon-default flex items-center gap-1.5 text-xs'>
                        <Loader2 className='size-3 animate-spin' />
                        {t('search.searching')}
                    </div>
                )}
                {status === 'completed' && (
                    <div className='text-app-sidebar-icon-default text-xs'>
                        {totalMatches > 0 ? t('search.matchCount', { count: totalMatches, files: results.length }) : t('search.noMatches')}
                    </div>
                )}
                {status === 'completed' && isTruncated && (
                    <div className='text-panel-match-highlight text-xs'>{t('search.truncated', { limit: SEARCH_MATCH_LIMIT })}</div>
                )}
            </div>

            {view === 'results' ? (
                <SearchResultsList
                    className='flex-1'
                    results={results}
                    onOpenMatch={onOpenMatch}
                    selection={
                        replaceOpen
                            ? { excludedPaths, onToggleSelect: (path) => setExcludedPaths((current) => toggleInSet(current, path)) }
                            : undefined
                    }
                />
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
                            {t('search.noResults')}
                        </div>
                    )}
                    {view === 'failed' && (
                        <div className='text-app-sidebar-icon-default flex h-full w-full items-center justify-center px-4 text-center text-xs'>
                            {t('search.failed')}
                        </div>
                    )}
                </ScrollContainer>
            )}

            <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
                <AlertDialogContent size='sm'>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('search.replaceConfirmTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('search.replaceConfirmDescription', { files: selectedGroups.length, matches: selectedMatchCount })}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={handleConfirmReplace}>{t('common.confirm')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    )
}
