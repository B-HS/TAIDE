import type { FC } from 'react'
import { useRef, useState } from 'react'
import { CircleCheck, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useVirtualizer } from '@tanstack/react-virtual'
import { ProblemRow } from '@features/problems/problem-row'
import type { ProblemGroup } from '@features/problems/problem-list-rows'
import { buildProblemListRows, PROBLEM_LIST_ROW_HEIGHT_PX } from '@features/problems/problem-list-rows'
import type { ProblemSeverity } from '@features/problems/problem-severity'
import { ProblemSeverityFilter } from '@features/problems/problem-severity-filter'
import { toggleInSet } from '@shared/lib/set'
import { FileGroupHeader } from '@shared/ui/file-group-header'
import { IconButton } from '@shared/ui/icon-button'
import { OverlayScrollbar } from '@shared/scroll/overlay-scrollbar'

const PROBLEM_LIST_OVERSCAN = 12

type ProblemsPanelProps = {
    groups: ProblemGroup[]
    hasAnyProblem: boolean
    counts: Record<ProblemSeverity, number>
    activeSeverities: Record<ProblemSeverity, boolean>
    onToggleSeverity: (severity: ProblemSeverity) => void
    onOpenProblem: (path: string, line: number, column: number) => void
    onClose: () => void
}

/**
 * Owns its own scroll viewport rather than nesting a `ScrollContainer`: the virtualizer needs the
 * scrolling element to measure against, and only the rows inside that window are mounted (the same
 * shape `search-results-list.tsx` uses). The empty state keeps the plain centered layout — there is
 * nothing to virtualize there.
 */
export const ProblemsPanel: FC<ProblemsPanelProps> = ({
    groups,
    hasAnyProblem,
    counts,
    activeSeverities,
    onToggleSeverity,
    onOpenProblem,
    onClose,
}) => {
    const viewportRef = useRef<HTMLDivElement>(null)

    const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(new Set())

    const rows = buildProblemListRows(groups, collapsedPaths)

    const rowVirtualizer = useVirtualizer({
        count: rows.length,
        getScrollElement: () => viewportRef.current,
        estimateSize: () => PROBLEM_LIST_ROW_HEIGHT_PX,
        overscan: PROBLEM_LIST_OVERSCAN,
        getItemKey: (index) => rows[index].id,
    })

    const { t } = useTranslation()

    return (
        <div className='bg-panel-background flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border flex shrink-0 items-center justify-between gap-2 border-b px-2 py-1.5'>
                <div className='flex min-w-0 items-center gap-2'>
                    <span className='text-app-foreground shrink-0 text-xs font-medium'>{t('problems.title')}</span>
                    <ProblemSeverityFilter counts={counts} active={activeSeverities} onToggle={onToggleSeverity} />
                </div>
                <IconButton
                    label={t('common.close')}
                    icon={<X className='size-3.5' />}
                    onClick={onClose}
                    side='bottom'
                    className='text-app-sidebar-icon-default hover:bg-explorer-item-hover flex size-5 shrink-0 items-center justify-center rounded-sm'
                />
            </div>

            {rows.length === 0 ? (
                <div className='text-app-sidebar-icon-default flex min-h-0 flex-1 flex-col items-center justify-center gap-2 px-4 text-center text-xs'>
                    <CircleCheck className='size-5 opacity-60' />
                    {t(hasAnyProblem ? 'problems.emptyFiltered' : 'problems.empty')}
                </div>
            ) : (
                <div className='relative min-h-0 flex-1'>
                    <div ref={viewportRef} className='scrollbar-hidden h-full w-full overflow-x-hidden overflow-y-auto'>
                        <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
                            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                                const row = rows[virtualRow.index]

                                return (
                                    <div
                                        key={virtualRow.key}
                                        data-index={virtualRow.index}
                                        ref={rowVirtualizer.measureElement}
                                        style={{
                                            position: 'absolute',
                                            top: 0,
                                            left: 0,
                                            width: '100%',
                                            transform: `translateY(${virtualRow.start}px)`,
                                        }}>
                                        {row.kind === 'group' ? (
                                            <FileGroupHeader
                                                path={row.path}
                                                count={row.problemCount}
                                                expanded={!row.collapsed}
                                                onToggle={() => setCollapsedPaths((current) => toggleInSet(current, row.path))}
                                            />
                                        ) : (
                                            <ProblemRow
                                                problem={row.problem}
                                                onClick={() => onOpenProblem(row.path, row.problem.line, row.problem.column)}
                                            />
                                        )}
                                    </div>
                                )
                            })}
                        </div>
                    </div>
                    <OverlayScrollbar viewportRef={viewportRef} orientation='vertical' />
                </div>
            )}
        </div>
    )
}
