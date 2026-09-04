import type { ProjectDisplayPatch } from '@shared/api/bindings'

/**
 * Longest short label the sidebar will draw for a project, counted in Unicode codepoints rather
 * than UTF-16 units so a label is measured the way a reader sees it. Mirrors
 * `domain::project::service::DISPLAY_LABEL_MAX_CODEPOINTS`, which rejects anything longer — keeping
 * the same number here is what makes the dialog unable to compose a value the backend would refuse.
 */
export const PROJECT_LABEL_MAX_CODEPOINTS = 4

/**
 * Font-size class for a label of N codepoints, indexed by N (index `0` exists only so the lookup
 * needs no special case). The sidebar button is 40px wide with `overflow-hidden`, and a CJK glyph is
 * roughly as wide as the font size, so four full-width characters only fit once the type shrinks —
 * a fixed `text-xs` would clip the fourth character instead. Index `PROJECT_LABEL_MAX_CODEPOINTS` is
 * the last entry: `resolveProjectLabelClassName` clamps to it, so a longer label (a record written
 * before the limit existed) still lands on the smallest step rather than off the end.
 */
export const PROJECT_LABEL_CLASS_BY_LENGTH = ['text-base', 'text-base', 'text-sm', 'text-[11px]', 'text-[9px] tracking-tight'] as const

/**
 * Color tokens a project display may name, mirroring `domain::project::service`'s
 * `DISPLAY_COLOR_TOKENS` allow-list and the theme system's `graph.lane1..lane12`. Every bundled
 * theme already defines all twelve, so the sidebar renders one as `var(--taide-graph-laneN)` with no
 * new theme token (and no second Rust/TS token list to keep in sync — `docs/theme-system.md`).
 */
export const PROJECT_COLOR_TOKENS = [
    'lane1',
    'lane2',
    'lane3',
    'lane4',
    'lane5',
    'lane6',
    'lane7',
    'lane8',
    'lane9',
    'lane10',
    'lane11',
    'lane12',
] as const

export type ProjectColorToken = (typeof PROJECT_COLOR_TOKENS)[number]

/**
 * CSS custom-property prefix a {@link ProjectColorToken} is appended to — the same variables
 * `commit-graph.tsx` reads inline for git lane colors (`src/shared/styles/global.css`).
 */
export const PROJECT_COLOR_VAR_PREFIX = '--taide-graph-'

/**
 * The patch that clears every display axis at once, backing the context menu's "reset to default".
 * `''` (not `null`) on each axis is the clear signal in the settings domain's clearable-string
 * convention that `ProjectDisplayPatch` follows — `null` would mean "leave this axis alone", which
 * would reset nothing.
 */
export const CLEARED_PROJECT_DISPLAY_PATCH: ProjectDisplayPatch = { icon: '', label: '', color: '' }
