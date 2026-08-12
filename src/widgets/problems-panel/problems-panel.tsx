import type { FC } from 'react'
import { useState } from 'react'
import { CircleCheck, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { ProblemRowData } from '@features/problems/problem-row'
import { ProblemRow } from '@features/problems/problem-row'
import type { ProblemSeverity } from '@features/problems/problem-severity'
import { ProblemSeverityFilter } from '@features/problems/problem-severity-filter'
import { FileGroupHeader } from '@shared/ui/file-group-header'
import { IconButton } from '@shared/ui/icon-button'
import { ScrollContainer } from '@shared/scroll/scroll-container'

export type ProblemGroup = {
    path: string
    problems: ProblemRowData[]
}

type ProblemsPanelProps = {
    groups: ProblemGroup[]
    hasAnyProblem: boolean
    counts: Record<ProblemSeverity, number>
    activeSeverities: Record<ProblemSeverity, boolean>
    onToggleSeverity: (severity: ProblemSeverity) => void
    onOpenProblem: (path: string, line: number, column: number) => void
    onClose: () => void
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

export const ProblemsPanel: FC<ProblemsPanelProps> = ({
    groups,
    hasAnyProblem,
    counts,
    activeSeverities,
    onToggleSeverity,
    onOpenProblem,
    onClose,
}) => {
    const { t } = useTranslation()
    const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(new Set())

    const hasVisibleGroups = groups.length > 0

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

            <ScrollContainer className='min-h-0 flex-1'>
                {!hasVisibleGroups && (
                    <div className='text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-2 px-4 text-center text-xs'>
                        <CircleCheck className='size-5 opacity-60' />
                        {t(hasAnyProblem ? 'problems.emptyFiltered' : 'problems.empty')}
                    </div>
                )}
                {hasVisibleGroups &&
                    groups.map((group) => {
                        const collapsed = collapsedPaths.has(group.path)
                        return (
                            <div key={group.path}>
                                <FileGroupHeader
                                    path={group.path}
                                    count={group.problems.length}
                                    expanded={!collapsed}
                                    onToggle={() => setCollapsedPaths((current) => toggleInSet(current, group.path))}
                                />
                                {!collapsed &&
                                    group.problems.map((problem) => (
                                        <ProblemRow
                                            key={`${group.path}:${problem.line}:${problem.column}:${problem.message}`}
                                            problem={problem}
                                            onClick={() => onOpenProblem(group.path, problem.line, problem.column)}
                                        />
                                    ))}
                            </div>
                        )
                    })}
            </ScrollContainer>
        </div>
    )
}
