import type { FC, PropsWithChildren } from 'react'
import { useEffect, useEffectEvent, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { QueryClient } from '@tanstack/react-query'
import { useQueryClient } from '@tanstack/react-query'
import type { ProjectId, ProjectLayout, ProjectRef } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS, TASK_COMPLETION_NOTIFY_MIN_DURATION_MS } from '@shared/constants/notification'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { i18next } from '@shared/i18n/i18n'
import { formatDurationShort } from '@shared/lib/format-duration'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import type { AgentCompletionEvaluation, AgentWorkingSinceMap } from '@shared/lib/native-notification-gate'
import { evaluateAgentCompletions, shouldNotifyTaskCompletion } from '@shared/lib/native-notification-gate'
import type { ProjectLayoutEntry } from '@shared/lib/notification-text'
import {
    findTerminalSessionOwner,
    findTerminalTabTitle,
    joinNotificationBody,
    notificationBodyFragment,
    resolveProjectNotificationTitle,
} from '@shared/lib/notification-text'
import { openNotificationSystemSettings } from '@entities/notification/notification.ipc'
import { notifyNative, subscribeNativeNotificationDelivered } from '@entities/notification/notify'

type AgentCompletion = AgentCompletionEvaluation['completed'][number]

/**
 * What the body of an agent notification says, now that the title is the project rather than the
 * event (batch 5 contract §1.F). "claude" alone answered none of the three questions a user who
 * walked away has — which agent, whether it finished or is blocked on them, and how long it took —
 * and the second of those is the one that decides whether they have to come back right now.
 *
 * `awaitingInput` deliberately carries no duration: a permission prompt is not a result, and the
 * time spent so far is not what the user needs to decide whether to answer it. Whether the wait is
 * a permission request or a question cannot be told apart from the activity alone, so the phrase
 * names both (a dedicated category and the distinction itself are wave 3 work).
 *
 * Reads `i18next` directly for the same reason `git.query.ts` does — the string is consumed by the
 * OS notification center, not rendered, so it must not be bound to a React render.
 */
export const buildAgentCompletionBody = (agent: AgentCompletion, tabTitle: string | null) => {
    const tab = notificationBodyFragment(tabTitle)
    if (agent.activity === 'awaitingInput') return i18next.t('notification.agentAwaitingInputBody', { agent: agent.name, tab })
    return i18next.t('notification.agentFinishedBody', { agent: agent.name, duration: formatDurationShort(agent.workedForMs), tab })
}

/**
 * What the body of a finished-command notification says. The exit code is the whole success/failure
 * signal once the title is the project name, so it is included whenever the shell reported one —
 * `0` included, since "exit code 0" is what tells the user the build they walked away from passed.
 * A shell that reported no code at all (`exitCode: null`) contributes nothing rather than an
 * invented value, which is what {@link joinNotificationBody} drops.
 */
export const buildTaskCompletionBody = (input: { target: string; exitCode: number | null; durationMs: number }) =>
    joinNotificationBody([
        input.target,
        input.exitCode === null ? null : i18next.t('notification.exitCode', { code: input.exitCode }),
        formatDurationShort(input.durationMs),
    ])

/**
 * Every open project's cached layout, which is what a `terminal:command-finished` has to be matched
 * against to learn whose terminal it was ({@link findTerminalSessionOwner}). Built from the project
 * list rather than by scanning the cache for `LAYOUT.*` keys so nothing here depends on the shape
 * of a query key, and so a closed project's stale layout cannot claim a session.
 */
const readCachedProjectLayouts = (queryClient: QueryClient): ProjectLayoutEntry[] =>
    (queryClient.getQueryData<ProjectRef[]>(QUERY_KEY.PROJECT.LIST) ?? []).map((project) => ({
        projectId: project.id,
        layout: queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(project.id)),
    }))

/**
 * Turns the backend broadcasts that have no in-app completion UI of their own — agent activity,
 * finished shell commands and language-server installs — into OS notifications, and says once per
 * session where to look if those notifications never appear.
 *
 * Mounted only in the main window's provider tree (`app.tsx`). `notifyNative` refuses to forward
 * from anywhere else anyway, but keeping the subscription itself out of auxiliary windows means the
 * per-agent timing state below exists once rather than once per window, each with its own
 * independently drifting view of who started working when.
 *
 * The agent path needs that state because `agent:state-changed` is a snapshot of the project's
 * whole roster with no history: "this agent just finished a long run" is only visible as the
 * difference between two payloads (`evaluateAgentCompletions`). The other two need none — a
 * `done`/`failed` phase and a `terminal:command-finished` are each already terminal on their own,
 * the latter carrying the runtime Rust measured for it.
 */
export const NativeNotificationProvider: FC<PropsWithChildren> = ({ children }) => {
    /**
     * Keyed by project first because a payload only ever carries one project's agents: folding
     * every project into one map would make each event look like "every other project's agents
     * disappeared" and drop their in-flight timings. Each project's entry is dropped when that
     * project closes — a closed project emits no further `agent:state-changed`, so its entry would
     * otherwise sit here for the rest of the session with nothing able to clear it.
     */
    const workingSinceByProjectRef = useRef<Record<ProjectId, AgentWorkingSinceMap>>({})
    const hasAnnouncedDeliveryRef = useRef(false)

    const { t } = useTranslation()
    const queryClient = useQueryClient()

    /**
     * The title line every project-scoped notification now shows. Read from the `PROJECT.LIST`
     * cache at send time rather than subscribed to: this provider renders once for the session and
     * has no use for a re-render when the roster changes, and a notification that arrives before
     * that query has answered falls back to the event's own title.
     */
    const projectNotificationTitle = (projectId: ProjectId | null, fallbackTitle: string) =>
        resolveProjectNotificationTitle({
            projects: queryClient.getQueryData<ProjectRef[]>(QUERY_KEY.PROJECT.LIST),
            projectId,
            fallbackTitle,
        })

    const announceFirstDelivery = useEffectEvent(() => {
        if (hasAnnouncedDeliveryRef.current) return
        hasAnnouncedDeliveryRef.current = true
        toast.info(t('notification.enableHint'), {
            action: {
                label: t('settings.notificationsOpenSystemSettings'),
                onClick: () => void openNotificationSystemSettings().catch((error: unknown) => toast.error(describeIpcError(error))),
            },
        })
    })

    useTauriEvent(events.agentStateChanged, ({ payload }) => {
        const { workingSince, completed } = evaluateAgentCompletions({
            workingSince: workingSinceByProjectRef.current[payload.projectId] ?? {},
            agents: payload.agents,
            nowMs: Date.now(),
            minWorkingMs: AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS,
        })
        workingSinceByProjectRef.current[payload.projectId] = workingSince
        if (completed.length === 0) return

        const title = projectNotificationTitle(payload.projectId, t('notification.agentCompleted'))
        const layout = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(payload.projectId))
        for (const agent of completed)
            void notifyNative({
                category: 'agentCompleted',
                title,
                body: buildAgentCompletionBody(agent, findTerminalTabTitle(layout, agent.sessionId)),
            })
    })

    useTauriEvent(events.projectClosed, ({ payload }) => {
        delete workingSinceByProjectRef.current[payload.projectId]
    })

    /**
     * The task-completion half of the channel (batch 4 contract §A.2-5, rewired by review F-1). The
     * shell's OSC 133 `D` is still the only thing that knows a command ended, but it is now read on
     * the pty reader thread rather than in the terminal's own xterm tracker: that tracker is
     * unmounted with its tab whenever the terminal goes to the background, which is exactly when a
     * long command needs announcing. Listening here also covers terminals in auxiliary windows,
     * which the facade's main-window-only rule used to leave silent (contract §3.4-2).
     *
     * A non-zero exit is reported under the `error` category so the "Failures" switch governs every
     * failure notification in one place.
     */
    useTauriEvent(events.terminalCommandFinished, ({ payload }) => {
        if (!shouldNotifyTaskCompletion({ durationMs: payload.durationMs, minDurationMs: TASK_COMPLETION_NOTIFY_MIN_DURATION_MS })) return
        const succeeded = payload.exitCode === null || payload.exitCode === 0
        const owner = findTerminalSessionOwner(readCachedProjectLayouts(queryClient), payload.sessionId)
        void notifyNative({
            category: succeeded ? 'taskCompleted' : 'error',
            title: projectNotificationTitle(
                owner?.projectId ?? null,
                t(succeeded ? 'notification.taskCompletedSucceeded' : 'notification.taskCompletedFailed'),
            ),
            body: buildTaskCompletionBody({
                target: owner?.tabTitle ?? payload.cwd ?? t('terminal.title'),
                exitCode: payload.exitCode,
                durationMs: payload.durationMs,
            }),
        })
    })

    useTauriEvent(events.lspInstallProgress, ({ payload }) => {
        if (payload.phase !== 'done' && payload.phase !== 'failed') return
        const succeeded = payload.phase === 'done'
        void notifyNative({
            category: succeeded ? 'lspInstall' : 'error',
            title: t(succeeded ? 'notification.lspInstallSucceeded' : 'notification.lspInstallFailed'),
            body: payload.message ?? payload.serverId,
        })
    })

    useEffect(() => subscribeNativeNotificationDelivered(announceFirstDelivery), [])

    return children
}
