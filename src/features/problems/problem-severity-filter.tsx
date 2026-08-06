import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { ProblemSeverity } from '@features/problems/problem-severity'
import { PROBLEM_SEVERITIES, PROBLEM_SEVERITY_COLOR_CLASS, PROBLEM_SEVERITY_ICON } from '@features/problems/problem-severity'
import { cn } from '@shared/lib/cn'

type ProblemSeverityFilterProps = {
    counts: Record<ProblemSeverity, number>
    active: Record<ProblemSeverity, boolean>
    onToggle: (severity: ProblemSeverity) => void
}

export const ProblemSeverityFilter: FC<ProblemSeverityFilterProps> = ({ counts, active, onToggle }) => {
    const { t } = useTranslation()

    return (
        <div role='group' aria-label={t('problems.filterAriaLabel')} className='flex items-center gap-1'>
            {PROBLEM_SEVERITIES.map((severity) => {
                const Icon = PROBLEM_SEVERITY_ICON[severity]
                return (
                    <button
                        key={severity}
                        type='button'
                        aria-pressed={active[severity]}
                        aria-label={t(`problems.severity.${severity}`)}
                        onClick={() => onToggle(severity)}
                        className={cn(
                            'flex items-center gap-1 rounded-sm px-1.5 py-0.5 text-xs',
                            active[severity]
                                ? 'bg-explorer-item-selected text-app-foreground'
                                : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover opacity-60',
                        )}>
                        <Icon className={cn('size-3', PROBLEM_SEVERITY_COLOR_CLASS[severity])} />
                        <span className='tabular-nums'>{counts[severity]}</span>
                    </button>
                )
            })}
        </div>
    )
}
