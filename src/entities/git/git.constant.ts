export const TOGGLE_BLAME_MONACO_ACTION_ID = 'taide.toggleBlame'
export const OPEN_FILE_HISTORY_MONACO_ACTION_ID = 'taide.openFileHistory'

/** Number of leading hex characters of a commit id shown as its short hash (commit graph, file history, commit detail panel). */
export const COMMIT_SHORT_HASH_LENGTH = 7

/**
 * Height of the SCM panel's commit-graph pane before the user has ever dragged it, and the fallback
 * for a `Settings` payload that predates `gitGraphPanelSizePx`. Mirrors Rust's
 * `DEFAULT_GIT_GRAPH_PANEL_SIZE_PX` so a cold start and a settings-less render agree.
 */
export const DEFAULT_GIT_GRAPH_PANEL_SIZE_PX = 240
