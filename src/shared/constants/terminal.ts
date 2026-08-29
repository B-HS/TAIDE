export const HIGH_WATER_BYTES = 512 * 1024

export const LOW_WATER_BYTES = 64 * 1024

export const DEFAULT_SCROLLBACK = 10_000

export const MIN_FONT_SIZE = 6

export const DEFAULT_FONT_SIZE = 13

export const RESIZE_DEBOUNCE_MS = 16

/**
 * What `terminal_sessions` reports as a session's `shell` when the spawn options carried no explicit
 * shell (the default login shell path) — mirrors `domain::terminal::commands::pty_spawn`'s
 * `opts.shell.unwrap_or_else(|| "default".to_string())`. Needed because `terminal-session.tsx` writes
 * the session it just spawned straight into the `TERMINAL.SESSIONS` cache (audit §4-B A6) and that
 * entry has to be indistinguishable from the one a later `terminal_sessions` fetch would return;
 * the value is not part of the specta-generated surface, so mirroring the literal is the only way to
 * keep the two in step.
 */
export const DEFAULT_SHELL_LABEL = 'default'
