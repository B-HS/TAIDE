import type { ProblemRowData } from '@features/problems/problem-row'

export type ProblemGroup = {
    path: string
    problems: ProblemRowData[]
}

export type ProblemListRow =
    | { kind: 'group'; id: string; path: string; problemCount: number; collapsed: boolean }
    | { kind: 'problem'; id: string; path: string; problem: ProblemRowData }

/**
 * Both row kinds are a single `text-xs` line box (16px) inside `py-0.5` (2px each side): the file
 * group header, and a problem row whose message is `truncate`d rather than wrapped. The virtualizer
 * re-measures every mounted row anyway, so this only has to be right for rows that have never been
 * on screen.
 */
export const PROBLEM_LIST_ROW_HEIGHT_PX = 20

/**
 * Flattens the per-file problem groups into one linear row list — the shape a virtualizer needs,
 * since it addresses items by a single index and cannot descend into nested per-file children.
 * A collapsed group keeps its header and contributes none of its problems, exactly as before.
 *
 * The panel previously rendered every group and every problem into the DOM at once (audit §1-12),
 * which on a large TypeScript project mid-refactor means thousands of rows re-created on every
 * marker change — and monaco pushes markers for the whole project, not just open files.
 *
 * Row ids are unique by construction: a file can legitimately carry two markers with identical
 * severity, position and message (two servers reporting the same thing), so the problem's index
 * within its group — not its contents — is what disambiguates.
 */
export const buildProblemListRows = (groups: ProblemGroup[], collapsedPaths: ReadonlySet<string>): ProblemListRow[] => {
    const rows: ProblemListRow[] = []

    for (const group of groups) {
        const collapsed = collapsedPaths.has(group.path)
        rows.push({ kind: 'group', id: `group:${group.path}`, path: group.path, problemCount: group.problems.length, collapsed })
        if (collapsed) continue

        group.problems.forEach((problem, index) => rows.push({ kind: 'problem', id: `problem:${group.path}:${index}`, path: group.path, problem }))
    }

    return rows
}
