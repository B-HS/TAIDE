const pendingBySessionId = new Map<string, Promise<unknown>>()

/**
 * Runs `write` after every write already queued for `sessionId`, so bytes reach the child's stdin in
 * the order the caller issued them.
 *
 * `pty_write` stopped being ordered by its own call order when the blocking write moved to the
 * blocking pool (contract §3 S4-(3), audit §2 M-6): the command's first poll now only clones the
 * writer handle and dispatches, so two invocations issued in the same tick become two independent
 * blocking tasks racing for the writer mutex, and the scheduler decides which wins. That shape is
 * reachable, not theoretical — `shared/lib/bridge/terminal-write-bridge.ts` flushes a whole queue of
 * buffered input (a task runner's command, "Run Selected Text", anything typed before the first
 * spawn resolved) in one synchronous loop, and each entry is fired without awaiting the previous.
 *
 * A rejected write must not stall the queue, so the next write is chained onto both outcomes; the
 * tail entry deletes itself once settled, so a session that stops writing leaves nothing behind.
 */
export const enqueueSessionWrite = <T>(sessionId: string, write: () => Promise<T>) => {
    const previous = pendingBySessionId.get(sessionId) ?? Promise.resolve()
    const next = previous.then(write, write)
    pendingBySessionId.set(sessionId, next)
    void next
        .catch(() => undefined)
        .then(() => {
            if (pendingBySessionId.get(sessionId) === next) pendingBySessionId.delete(sessionId)
        })
    return next
}
