/**
 * Splits one chunk of an attached pty stream into the part that counts toward the write backlog
 * (`terminal-flow-control.ts`) and the part that is replayed scrollback, which must not.
 *
 * `pty_attach` replays up to a full 2MB ring through the same channel the live output uses, so a
 * terminal tab switch handed `evaluateFlowControl` a backlog far past `HIGH_WATER` and paused a
 * perfectly healthy child process over output the user had already seen. The attach's
 * `replayBytes` is the budget: bytes are charged against it in arrival order, and only what is left
 * over is live output worth pausing for.
 *
 * The budget is consumed byte-wise rather than chunk-wise so a chunk that runs past the end of the
 * replay still contributes its live remainder, and a negative budget (nothing left, or a caller
 * that never learned one) degrades to "count everything".
 */
export const consumeReplayBudget = (budget: number, chunkBytes: number) => {
    const remainingBefore = Math.max(budget, 0)
    const consumed = Math.min(remainingBefore, chunkBytes)
    return { countedBytes: chunkBytes - consumed, remainingBudget: remainingBefore - consumed }
}
