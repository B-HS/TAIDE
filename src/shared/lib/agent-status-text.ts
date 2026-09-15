import type { AgentActivity, BlockedReason } from '@shared/api/bindings'

/**
 * What a blocked agent is actually waiting on, one key per {@link BlockedReason}. These replace the
 * generic `agent.status.awaitingInput` line rather than being appended to it: "Needs your input"
 * plus "Waiting for permission" says the same thing twice in a tooltip that already has one line
 * per agent, and the reason alone answers the question the generic phrase left open.
 *
 * `dialog` stays vague on purpose — it is read off the screen, so all it proves is that some
 * dialog is up (`domain::agent::service::blocked_reason`).
 */
const BLOCKED_REASON_LABEL_KEY: Record<BlockedReason, string> = {
    permission: 'agent.blocked.permission',
    question: 'agent.blocked.question',
    dialog: 'agent.blocked.dialog',
}

const AWAITING_INPUT_ACTIVITY: AgentActivity = 'awaitingInput'

/**
 * The catalog key describing one agent's current state, which is what the project icon tooltip, the
 * terminal tab tooltip and the icon button's `aria-label` all say about it.
 *
 * A reason is only ever attached to `awaitingInput`: the backend clears it the moment the latch
 * behind that activity is released, and the hysteresis window (`classify_session` step 8) can hand
 * back an `awaitingInput` with no reason at all, so the generic status line has to stay reachable
 * rather than being replaced by a "blocked for unknown reasons" phrasing.
 */
export const agentStatusLabelKey = (activity: AgentActivity, blockedReason: BlockedReason | null | undefined) => {
    if (activity === AWAITING_INPUT_ACTIVITY && blockedReason) return BLOCKED_REASON_LABEL_KEY[blockedReason]
    return `agent.status.${activity}`
}
