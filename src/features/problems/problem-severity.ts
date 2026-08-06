import { CircleX, Info, Lightbulb, TriangleAlert } from 'lucide-react'
import { monaco } from '@shared/lib/monaco/setup'

export const PROBLEM_SEVERITIES = ['error', 'warning', 'info', 'hint'] as const

export type ProblemSeverity = (typeof PROBLEM_SEVERITIES)[number]

export const toProblemSeverity = (severity: monaco.MarkerSeverity): ProblemSeverity => {
    if (severity === monaco.MarkerSeverity.Error) return 'error'
    if (severity === monaco.MarkerSeverity.Warning) return 'warning'
    if (severity === monaco.MarkerSeverity.Info) return 'info'
    return 'hint'
}

export const PROBLEM_SEVERITY_ICON = { error: CircleX, warning: TriangleAlert, info: Info, hint: Lightbulb } as const

export const PROBLEM_SEVERITY_COLOR_CLASS = {
    error: 'text-status-error',
    warning: 'text-status-warning',
    info: 'text-status-info',
    hint: 'text-app-sidebar-icon-default',
} as const satisfies Record<ProblemSeverity, string>
