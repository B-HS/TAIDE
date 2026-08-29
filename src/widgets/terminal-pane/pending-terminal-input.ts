/**
 * Upper bound on the keystrokes {@link appendPendingTerminalInput} holds while a terminal tab's very
 * first `pty_spawn` is still in flight. A spawn resolves in well under a second (it writes the shell
 * integration scripts, opens the pty and forks the child), so this only ever has to cover typeahead
 * plus a paste that lands in that window — generous for both, while still bounding the buffer if a
 * spawn hangs. Cross-widget writers ("Run Selected Text", the task runner) never reach here: they go
 * through `terminal-write-bridge`, which has its own queue for exactly the same pre-spawn window.
 */
export const TERMINAL_PENDING_INPUT_MAX_CHARS = 4096

/**
 * Accumulates terminal input typed before the pty session exists, so it can be written the moment
 * the spawn resolves instead of being dropped on the floor (audit §4-B C14: the first keystrokes
 * into a freshly opened terminal tab silently vanished, since `handleWrite` had no session to write
 * to yet). Overflow drops from the *front*, matching `terminal-write-bridge`'s queue policy — what
 * the user typed most recently is what the shell should see.
 */
export const appendPendingTerminalInput = (pending: string, data: string, maxChars: number = TERMINAL_PENDING_INPUT_MAX_CHARS) => {
    const next = `${pending}${data}`
    if (next.length <= maxChars) return next
    return next.slice(next.length - maxChars)
}
