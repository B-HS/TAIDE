import { DEFAULT_SCROLLBACK_BYTES, MAX_SCROLLBACK_BYTES, SCROLLBACK_BYTES_PER_LINE_ESTIMATE } from '@shared/constants/terminal'

/**
 * Converts `Settings.terminalScrollback` — a line count, which is the only unit the settings UI ever
 * offered — into the `PtySpawnOptions.scrollbackBytes` budget a pty's `ScrollbackRing` is built
 * with, clamped to the same `DEFAULT_SCROLLBACK_BYTES..=MAX_SCROLLBACK_BYTES` window Rust's
 * `resolve_scrollback_bytes` applies. Clamping on both sides is deliberate rather than redundant:
 * this side keeps the number the user sees in the settings screen honest about what it can buy,
 * while the backend still has to defend against a remote peer's `pty_spawn` (`domain::remote`).
 *
 * A missing setting resolves to the floor instead of `DEFAULT_SCROLLBACK` × the estimate, so a
 * window that has not fetched settings yet spawns with exactly the budget every spawn had before
 * this field existed rather than a silently different one.
 */
export const resolveScrollbackBytes = (scrollbackLines: number | null | undefined) => {
    if (typeof scrollbackLines !== 'number' || !Number.isFinite(scrollbackLines)) return DEFAULT_SCROLLBACK_BYTES
    const requested = Math.floor(scrollbackLines * SCROLLBACK_BYTES_PER_LINE_ESTIMATE)
    return Math.min(Math.max(requested, DEFAULT_SCROLLBACK_BYTES), MAX_SCROLLBACK_BYTES)
}
