import type { ProjectId, ProjectLayout, ProjectRef } from '@shared/api/bindings'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { resolveProjectDisplay } from '@shared/lib/project-display'

/**
 * What separates the pieces of a notification body — the same middle dot the catalog's own body
 * templates spell out, kept here so the fragments assembled in code cannot drift from them.
 */
const NOTIFICATION_BODY_SEPARATOR = ' · '

const isPresentText = (part: string | null | undefined): part is string => typeof part === 'string' && part.trim().length > 0

/**
 * Joins the pieces of a notification body, dropping the ones that turned out to be absent — a git
 * status cache that has not answered yet has no branch, a shell that reported no exit code has no
 * code. Filtering rather than interpolating an empty string is what keeps a missing piece from
 * leaving a dangling separator ("push complete · ").
 */
export const joinNotificationBody = (parts: readonly (string | null | undefined)[]) => parts.filter(isPresentText).join(NOTIFICATION_BODY_SEPARATOR)

/**
 * A trailing "` · something`" for the catalog templates that take an optional tail as a single
 * interpolation — i18next has no conditional segment, so the choice between "with a tab name" and
 * "without one" is made here and handed over as a value that is either the fragment or the empty
 * string, rather than as two near-identical catalog keys per message.
 */
export const notificationBodyFragment = (text: string | null | undefined) => (isPresentText(text) ? `${NOTIFICATION_BODY_SEPARATOR}${text}` : '')

/**
 * The project a completion notification belongs to, as the title line.
 *
 * Which project a background event came from is the first thing the user needs when several are
 * open — the event's own name ("Agent finished") is what the body now says. The display override
 * wins over the folder name because it is what the same project shows in the sidebar, so the two
 * cannot disagree; `fallbackTitle` (the event's own title) covers a project whose record is not in
 * the cache, which happens whenever a notification arrives before `project_list` has answered.
 *
 * Reads a `ProjectRef[]` the caller pulled out of the query cache rather than fetching: this runs
 * on a path the user has already walked away from, and a title is not worth an IPC round trip
 * (the same reasoning `git.query.ts` applies to the branch name it puts in a push notification).
 */
export const resolveProjectNotificationTitle = (input: {
    projects: readonly ProjectRef[] | undefined
    projectId: ProjectId | null
    fallbackTitle: string
}) => {
    const project = input.projectId === null ? undefined : input.projects?.find((candidate) => candidate.id === input.projectId)
    if (!project) return input.fallbackTitle
    return resolveProjectDisplay({ display: project.display }).label ?? project.name
}

/**
 * The title of the terminal tab running `sessionId`, or `null` when that session has no tab in this
 * layout — the detail that tells two agents in the same project apart. Auxiliary windows are
 * searched too ({@link collectAllPaneTabs}), since a background agent is exactly the one whose tab
 * was moved out of the way.
 */
export const findTerminalTabTitle = (layout: ProjectLayout | null | undefined, sessionId: string) => {
    if (!layout || sessionId.length === 0) return null
    const tab = collectAllPaneTabs(layout).find((candidate) => candidate.kind.kind === 'terminal' && candidate.kind.sessionId === sessionId)
    return tab?.title ?? null
}

export type ProjectLayoutEntry = { projectId: ProjectId; layout: ProjectLayout | null | undefined }

/**
 * Which project owns a terminal session, found by asking every cached layout whether it holds a tab
 * for it. `terminal:command-finished` carries only a `sessionId` — the pty reader thread that emits
 * it knows nothing about projects — so the tab that session belongs to is the only link back, and
 * the same lookup yields the tab title the body wants anyway. `null` when no cached layout claims
 * it (a session whose tab was already closed), which leaves the caller on the event's own title.
 */
export const findTerminalSessionOwner = (entries: readonly ProjectLayoutEntry[], sessionId: string) => {
    for (const entry of entries) {
        const tabTitle = findTerminalTabTitle(entry.layout, sessionId)
        if (tabTitle !== null) return { projectId: entry.projectId, tabTitle }
    }
    return null
}
