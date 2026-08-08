import type { FC, KeyboardEvent } from 'react'
import { useEffect, useRef, useState } from 'react'
import { CaseSensitive, ChevronRight, Loader2, Regex, ReplaceAll, Search, WholeWord, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { SearchMatchRowData } from '@features/search/search-match-row'
import { SearchMatchRow } from '@features/search/search-match-row'
import { cn } from '@shared/lib/cn'
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
import { FileGroupHeader } from '@shared/ui/file-group-header'
import { ScrollContainer } from '@shared/scroll/scroll-container'

export type SearchResultGroup = {
    path: string
    matches: SearchMatchRowData[]
}

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
    onSubmit: () => void
    isSearching: boolean
    totalMatches: number
    results: SearchResultGroup[]
    onOpenMatch: (path: string) => void
    onReplaceAll: (input: ReplaceAllInput) => void
    isReplacing: boolean
    scopePath: string | null
    onClearScope: () => void
    openReplace: boolean
    openNonce: number
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
    regex,
    onRegexChange,
    onSubmit,
    isSearching,
    totalMatches,
    results,
    onOpenMatch,
    onReplaceAll,
    isReplacing,
    scopePath,
    onClearScope,
    openReplace,
    openNonce,
}) => {
    const { t } = useTranslation()
    const queryInputRef = useRef<HTMLInputElement>(null)
    const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(new Set())
    const [replaceOpen, setReplaceOpen] = useState(false)
    const [replaceText, setReplaceText] = useState('')
    const [excludedPaths, setExcludedPaths] = useState<Set<string>>(new Set())
    const [confirmOpen, setConfirmOpen] = useState(false)
    const [appliedOpenNonce, setAppliedOpenNonce] = useState<number | null>(null)

    if (openNonce !== appliedOpenNonce) {
        setAppliedOpenNonce(openNonce)
        if (openReplace) setReplaceOpen(true)
    }

    const hasQuery = query.trim().length > 0
    const hasResults = results.length > 0
    const selectedGroups = results.filter((group) => !excludedPaths.has(group.path))
    const selectedMatchCount = selectedGroups.reduce((sum, group) => sum + group.matches.length, 0)
    const canReplaceAll = replaceOpen && selectedGroups.length > 0 && !isReplacing

    const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key !== 'Enter') return
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
                    <button
                        type='button'
                        aria-label={t('search.replaceToggle')}
                        aria-pressed={replaceOpen}
                        onClick={() => setReplaceOpen(!replaceOpen)}
                        className='text-app-sidebar-icon-default hover:bg-explorer-item-hover mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-sm'>
                        <ChevronRight className={cn('size-3.5 transition-transform', replaceOpen && 'rotate-90')} />
                    </button>
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
                            <button
                                type='button'
                                aria-label={t('search.caseSensitive')}
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
                                aria-label={t('search.wholeWord')}
                                aria-pressed={wholeWord}
                                onClick={() => onWholeWordChange(!wholeWord)}
                                className={cn(
                                    'flex size-6 shrink-0 items-center justify-center rounded-sm',
                                    wholeWord
                                        ? 'bg-explorer-item-selected text-app-foreground'
                                        : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover',
                                )}>
                                <WholeWord className='size-3.5' />
                            </button>
                            <button
                                type='button'
                                aria-label={t('search.regex')}
                                aria-pressed={regex}
                                onClick={() => onRegexChange(!regex)}
                                className={cn(
                                    'mr-1 flex size-6 shrink-0 items-center justify-center rounded-sm',
                                    regex
                                        ? 'bg-explorer-item-selected text-app-foreground'
                                        : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover',
                                )}>
                                <Regex className='size-3.5' />
                            </button>
                        </div>
                        {replaceOpen && (
                            <div className='bg-panel-input-background border-panel-input-border focus-within:border-app-focus-border flex items-center rounded-sm border'>
                                <input
                                    value={replaceText}
                                    onChange={(event) => setReplaceText(event.target.value)}
                                    placeholder={t('search.replacePlaceholder')}
                                    className='min-w-0 flex-1 bg-transparent px-2 py-1.5 text-xs outline-none'
                                />
                                <button
                                    type='button'
                                    aria-label={t('search.replaceAll')}
                                    disabled={!canReplaceAll}
                                    onClick={() => setConfirmOpen(true)}
                                    className='text-app-sidebar-icon-default hover:bg-explorer-item-hover mr-1 flex size-6 shrink-0 items-center justify-center rounded-sm disabled:pointer-events-none disabled:opacity-40'>
                                    <ReplaceAll className='size-3.5' />
                                </button>
                            </div>
                        )}
                    </div>
                </div>
                {scopePath && (
                    <div className='bg-explorer-item-selected text-app-foreground flex w-fit max-w-full items-center gap-1 rounded-sm px-1.5 py-0.5 text-xs'>
                        <span className='truncate'>{t('explorer.searchScopeLabel', { path: scopePath })}</span>
                        <button
                            type='button'
                            aria-label={t('common.close')}
                            onClick={onClearScope}
                            className='text-app-sidebar-icon-default hover:text-app-foreground shrink-0'>
                            <X className='size-3' />
                        </button>
                    </div>
                )}
                {isSearching && (
                    <div className='text-app-sidebar-icon-default flex items-center gap-1.5 text-xs'>
                        <Loader2 className='size-3 animate-spin' />
                        {t('search.searching')}
                    </div>
                )}
                {!isSearching && hasQuery && (
                    <div className='text-app-sidebar-icon-default text-xs'>
                        {totalMatches > 0 ? t('search.matchCount', { count: totalMatches, files: results.length }) : t('search.noMatches')}
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
                        {t('search.noResults')}
                    </div>
                )}
                {hasResults &&
                    results.map((group) => {
                        const collapsed = collapsedPaths.has(group.path)
                        return (
                            <div key={group.path}>
                                <FileGroupHeader
                                    path={group.path}
                                    count={group.matches.length}
                                    expanded={!collapsed}
                                    onToggle={() => setCollapsedPaths((current) => toggleInSet(current, group.path))}
                                    selected={replaceOpen ? !excludedPaths.has(group.path) : undefined}
                                    onToggleSelect={replaceOpen ? () => setExcludedPaths((current) => toggleInSet(current, group.path)) : undefined}
                                    selectAriaLabel={t('search.selectFile', { path: group.path })}
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
            </ScrollContainer>

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
