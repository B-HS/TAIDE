export const DEFAULT_RESIZER_THICKNESS = 1

export const MIN_RESIZER_THICKNESS = 0

export const MAX_RESIZER_THICKNESS = 8

export const RESIZE_HIT_TARGET_SIZE = { fine: 8, coarse: 20 } as const

/**
 * Smallest pixel size `pane-node-view.tsx` gives a `Panel`, and therefore the smallest a pane can
 * be after a split. Lives here rather than in that file because the terminal's context menu has to
 * answer "would a split in this direction fit?" *before* asking for one
 * (`terminal-split-availability.ts`): react-resizable-panels converts pixel `minSize` values to
 * percentages of the group and normalizes them when they cannot all be honoured, so a split into
 * two panes narrower than this is not refused — it is silently squashed. Both call sites must read
 * the same number for that pre-check to mean anything.
 */
export const MIN_PANEL_SIZE_PX = 120
