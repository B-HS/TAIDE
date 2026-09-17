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

/**
 * How many bytes of pty output one line of terminal scrollback is assumed to cost, used to convert
 * `Settings.terminalScrollback` (which the settings UI asks for in *lines*) into the byte budget
 * `PtySpawnOptions.scrollbackBytes` wants. The two units were never connected before: the slider
 * moved xterm's own display buffer while the Rust `ScrollbackRing` every tab switch replays from
 * stayed pinned at 2 MiB, so a user who raised the setting still lost everything past the last
 * 2 MiB the moment they switched tabs and back, with nothing saying so (audit wave 2 #18).
 *
 * 512 B is deliberately generous for an 80-column terminal — real build/test output carries ANSI
 * colour runs, OSC 133 prompt marks, and box-drawing multibyte glyphs, so a budget derived from the
 * bare column count would under-serve exactly the long logs the setting exists for. Being an
 * estimate is the whole reason both sides clamp rather than reject: 100,000 lines asks for 48.8 MiB
 * and lands on {@link MAX_SCROLLBACK_BYTES}, 100 lines asks for 50 KiB and lands on
 * {@link DEFAULT_SCROLLBACK_BYTES}.
 */
export const SCROLLBACK_BYTES_PER_LINE_ESTIMATE = 512

/** Mirrors Rust `domain::terminal::types::DEFAULT_SCROLLBACK_BYTES` — the floor of the clamp, and what a spawn that requests nothing gets. */
export const DEFAULT_SCROLLBACK_BYTES = 2 * 1024 * 1024

/** Mirrors Rust `domain::terminal::types::MAX_SCROLLBACK_BYTES` — the ceiling `resolve_scrollback_bytes` clamps an explicit request to. */
export const MAX_SCROLLBACK_BYTES = 32 * 1024 * 1024
