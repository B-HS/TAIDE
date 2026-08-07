import type { FC } from 'react'
import { useState } from 'react'
import { toast } from 'sonner'
import type { ProjectId } from '@shared/api/bindings'
import { useMonacoMarkers } from '@shared/hooks/use-monaco-markers'
import { useOpenTab } from '@entities/layout/layout.query'
import { requestReveal } from '@entities/editor/reveal-registry'
import type { ProblemRowData } from '@features/problems/problem-row'
import type { ProblemSeverity } from '@features/problems/problem-severity'
import { PROBLEM_SEVERITIES, toProblemSeverity } from '@features/problems/problem-severity'
import type { ProblemGroup } from '@widgets/problems-panel/problems-panel'
import { ProblemsPanel } from '@widgets/problems-panel/problems-panel'

type ProblemsPanelContainerProps = {
    projectId: ProjectId
    onClose: () => void
}

const fileNameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

const emptySeverityRecord = <T,>(value: T): Record<ProblemSeverity, T> =>
    Object.fromEntries(PROBLEM_SEVERITIES.map((severity) => [severity, value])) as Record<ProblemSeverity, T>

const compareProblems = (a: ProblemRowData, b: ProblemRowData) => a.line - b.line || a.column - b.column

export const ProblemsPanelContainer: FC<ProblemsPanelContainerProps> = ({ projectId, onClose }) => {
    const [activeSeverities, setActiveSeverities] = useState<Record<ProblemSeverity, boolean>>(emptySeverityRecord(true))

    const markers = useMonacoMarkers()
    const { mutate: openTab } = useOpenTab(projectId)

    const counts = emptySeverityRecord(0)
    const problemsByPath = new Map<string, ProblemRowData[]>()

    for (const marker of markers) {
        const severity = toProblemSeverity(marker.severity)
        counts[severity] += 1
        if (!activeSeverities[severity]) continue

        const path = marker.resource.fsPath
        const list = problemsByPath.get(path) ?? []
        list.push({ severity, line: marker.startLineNumber, column: marker.startColumn, message: marker.message, source: marker.source ?? null })
        problemsByPath.set(path, list)
    }

    const groups: ProblemGroup[] = [...problemsByPath.entries()]
        .map(([path, problems]) => ({ path, problems: problems.toSorted(compareProblems) }))
        .toSorted((a, b) => a.path.localeCompare(b.path))

    const handleOpenProblem = (path: string, line: number, column: number) => {
        requestReveal(path, line, column)
        openTab(
            { projectId, kind: { kind: 'file', path }, title: fileNameOf(path), target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
        )
    }

    return (
        <ProblemsPanel
            groups={groups}
            hasAnyProblem={markers.length > 0}
            counts={counts}
            activeSeverities={activeSeverities}
            onToggleSeverity={(severity) => setActiveSeverities((current) => ({ ...current, [severity]: !current[severity] }))}
            onOpenProblem={handleOpenProblem}
            onClose={onClose}
        />
    )
}
