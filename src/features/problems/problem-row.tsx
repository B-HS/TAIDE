import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { ProblemSeverity } from '@features/problems/problem-severity'
import { PROBLEM_SEVERITY_COLOR_CLASS, PROBLEM_SEVERITY_ICON } from '@features/problems/problem-severity'
import { cn } from '@shared/lib/cn'

export type ProblemRowData = {
    severity: ProblemSeverity
    line: number
    column: number
    message: string
    source: string | null
}

type ProblemRowProps = {
    problem: ProblemRowData
    onClick: () => void
}

const PROBLEM_ROW_INDENT_PX = 32

export const ProblemRow: FC<ProblemRowProps> = ({ problem, onClick }) => {
    const { t } = useTranslation()
    const Icon = PROBLEM_SEVERITY_ICON[problem.severity]

    return (
        <div
            role='button'
            tabIndex={0}
            onClick={onClick}
            onKeyDown={(event) => event.key === 'Enter' && onClick()}
            style={{ paddingLeft: PROBLEM_ROW_INDENT_PX }}
            className='hover:bg-explorer-item-hover flex cursor-default items-start gap-2 py-0.5 pr-2 text-xs select-none'>
            <Icon
                aria-label={t(`problems.severity.${problem.severity}`)}
                className={cn('mt-0.5 size-3.5 shrink-0', PROBLEM_SEVERITY_COLOR_CLASS[problem.severity])}
            />
            <span className='min-w-0 flex-1 truncate'>{problem.message}</span>
            {problem.source && <span className='text-app-sidebar-icon-default shrink-0'>{problem.source}</span>}
            <span className='text-app-sidebar-icon-default shrink-0 tabular-nums'>{`${problem.line}:${problem.column}`}</span>
        </div>
    )
}
