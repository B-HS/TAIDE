import type { AgentActivity } from '@shared/api/bindings'
import type { WindowContext } from '@shared/lib/window-context'

/**
 * Whether this JS realm is the one allowed to hand a completion event to the OS notification
 * center. Every realm hears the same backend broadcasts — `agent:state-changed` and
 * `lsp:install-progress` reach the main window, every auxiliary editor window and the remote
 * mirror alike — so without this gate a user with two windows open would get the same notification
 * twice, and the browser-based mirror would try to invoke a desktop-only command that the remote
 * dispatcher denies anyway (`remote/dispatch.rs`).
 *
 * The main window is the one realm guaranteed to exist for the whole session, which is why it owns
 * the channel rather than "whichever window saw the event first". Note this decides *who forwards*,
 * not *whether the user should be interrupted*: that second question is app-wide (is any TAIDE
 * window focused, are the category switches on) and is answered by Rust in
 * `domain::notification::service::decide_delivery`, since no single window can see the others.
 */
export const shouldForwardNativeNotification = (input: { windowKind: WindowContext['kind']; isRemoteMirror: boolean }) =>
    input.windowKind === 'main' && !input.isRemoteMirror

/** Activities that count as "the agent handed control back", the only transitions worth notifying about. */
const AGENT_COMPLETION_ACTIVITIES: readonly AgentActivity[] = ['idle', 'awaitingInput']

const WORKING_ACTIVITY: AgentActivity = 'working'

export type AgentCompletionCandidate = { sessionId: string; name: string; activity: AgentActivity }

/** `sessionId` → the `Date.now()` at which that agent was first seen `working` in the current stretch. */
export type AgentWorkingSinceMap = Readonly<Record<string, number>>

export type AgentCompletionEvaluation = {
    workingSince: AgentWorkingSinceMap
    completed: { sessionId: string; name: string; workedForMs: number }[]
}

/**
 * Folds one `agent:state-changed` payload into the "who has been working since when" map and
 * reports which agents just finished a long enough stretch to be worth a notification.
 *
 * `agent:state-changed` carries the project's whole agent roster as a snapshot with no history, so
 * "finished" only exists as the difference between two snapshots — hence the caller-held map. The
 * map is rebuilt from the incoming roster rather than mutated, which is also what evicts agents
 * whose process disappeared: an agent that vanishes mid-run (killed shell, closed terminal) is
 * dropped without notifying, since it never handed control back.
 *
 * Time is a parameter rather than read here so every threshold case is testable, and the map is
 * keyed per project by the caller — a payload only ever describes one project's agents, so folding
 * two projects into one map would drop the other project's in-flight agents on every event.
 */
export const evaluateAgentCompletions = (input: {
    workingSince: AgentWorkingSinceMap
    agents: readonly AgentCompletionCandidate[]
    nowMs: number
    minWorkingMs: number
}): AgentCompletionEvaluation => {
    const workingSince: Record<string, number> = {}
    const completed: AgentCompletionEvaluation['completed'] = []

    for (const agent of input.agents) {
        const startedAtMs = input.workingSince[agent.sessionId]
        if (agent.activity === WORKING_ACTIVITY) {
            workingSince[agent.sessionId] = startedAtMs ?? input.nowMs
            continue
        }
        if (startedAtMs === undefined || !AGENT_COMPLETION_ACTIVITIES.includes(agent.activity)) continue
        const workedForMs = input.nowMs - startedAtMs
        if (workedForMs < input.minWorkingMs) continue
        completed.push({ sessionId: agent.sessionId, name: agent.name, workedForMs })
    }

    return { workingSince, completed }
}

/**
 * Whether a finished shell command ran long enough to be worth an OS notification. A prompt that
 * returns instantly is something the user watched happen; the threshold is what separates "I ran
 * `ls`" from "I started a build and went to read something else".
 *
 * Only measured commands reach here at all: `terminal:command-finished` is emitted from the pty
 * reader thread's own OSC 133 `C`→`D` clock (`domain::terminal::commands::report_command_marker`),
 * which stays silent for a `D` whose `C` it never saw rather than reporting a duration it would have
 * to invent.
 */
export const shouldNotifyTaskCompletion = (input: { durationMs: number; minDurationMs: number }) => input.durationMs >= input.minDurationMs

/**
 * Caps one notification field at `maxCodePoints`, ending an over-long value with an ellipsis.
 *
 * Counted in code points rather than UTF-16 units so a body of emoji or CJK is cut where a reader
 * would see the cut, never mid-surrogate — a lone surrogate half is what would actually reach
 * `NSString` on macOS. Nothing downstream imposes a limit of its own: the plugin passes the string
 * through and macOS renders whatever it is given, so a failed LSP install's stderr would otherwise
 * arrive in the notification center in full.
 */
export const truncateNotificationText = (text: string, maxCodePoints: number) => {
    const codePoints = [...text]
    if (codePoints.length <= maxCodePoints) return text
    return `${codePoints.slice(0, maxCodePoints - 1).join('')}…`
}
