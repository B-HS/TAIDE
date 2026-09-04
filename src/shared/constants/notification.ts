/**
 * How long an agent has to stay `working` before its return to `idle`/`awaitingInput` is worth an
 * OS notification. Agents flip through `working` for a fraction of a second on every trivial turn
 * (a tool call, a file read), and notifying for those would train the user to ignore the whole
 * channel — the notification is only useful for the work the user walked away from. Ten seconds is
 * the same floor {@link TASK_COMPLETION_NOTIFY_MIN_DURATION_MS} applies to shell commands, so both
 * "something finished" sources answer to one idea of "long enough that you stopped watching".
 */
export const AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS = 10_000

/**
 * How long a shell command (OSC 133 `C`→`D`) has to run before its completion is worth an OS
 * notification — see {@link AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS} for why the two thresholds are
 * deliberately the same number rather than one shared constant: they gate unrelated event sources
 * and either could be retuned on its own.
 */
export const TASK_COMPLETION_NOTIFY_MIN_DURATION_MS = 10_000

/**
 * Upper bound, in code points, on the title and body handed to the OS notification center. Some
 * bodies are foreign text of unbounded length — an LSP install's own stderr, a `describeIpcError`
 * carrying a git remote's full rejection message — and neither the plugin nor macOS truncates it,
 * so a failed install can otherwise put a whole stack trace in a notification banner. Two lines'
 * worth is the amount a banner can actually show before the rest is invisible anyway.
 */
export const NOTIFICATION_TEXT_MAX_CODE_POINTS = 200
