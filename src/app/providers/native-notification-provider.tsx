import type { FC, PropsWithChildren } from 'react'
import { useEffect, useEffectEvent, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS, TASK_COMPLETION_NOTIFY_MIN_DURATION_MS } from '@shared/constants/notification'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import type { AgentWorkingSinceMap } from '@shared/lib/native-notification-gate'
import { evaluateAgentCompletions, shouldNotifyTaskCompletion } from '@shared/lib/native-notification-gate'
import { openNotificationSystemSettings } from '@entities/notification/notification.ipc'
import { notifyNative, subscribeNativeNotificationDelivered } from '@entities/notification/notify'

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
        for (const agent of completed) void notifyNative({ category: 'agentCompleted', title: t('notification.agentCompleted'), body: agent.name })
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
        void notifyNative({
            category: succeeded ? 'taskCompleted' : 'error',
            title: t(succeeded ? 'notification.taskCompletedSucceeded' : 'notification.taskCompletedFailed'),
            body: payload.cwd ?? t('terminal.title'),
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
