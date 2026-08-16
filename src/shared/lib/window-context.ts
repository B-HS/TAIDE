import type { ProjectId } from '@shared/api/bindings'

const AUXILIARY_QUERY_PARAM_PROJECT_ID = 'projectId'
const AUXILIARY_QUERY_PARAM_WINDOW_SLOT = 'windowSlot'

/**
 * Identifies which OS window this JS realm is running in — the main window (no query string) or
 * an auxiliary editor window (`editor-<n>`). `domain::window::commands::open_auxiliary_window`
 * (Wave I contract §3.1) opens auxiliary windows at `index.html?projectId=<id>&windowSlot=<n>`,
 * which is the only signal available client-side; Tauri never navigates a window's webview after
 * creation, so this is stable for the whole process lifetime.
 */
export type WindowContext = { kind: 'main' } | { kind: 'auxiliary'; projectId: ProjectId; windowSlot: number }

const MAIN_WINDOW_CONTEXT: WindowContext = { kind: 'main' }

/**
 * Pure parser over a `location.search`-shaped string, kept separate from {@link getWindowContext}
 * so it can run under `bun:test` — the runtime has no `window` global (same constraint
 * `keymap-context.test.ts` documents for `document`). Any query string missing either param, or
 * carrying a non-integer `windowSlot`, resolves to the main window rather than throwing — a
 * malformed URL should never strand a window unable to render anything.
 */
export const readWindowContext = (search: string): WindowContext => {
    const params = new URLSearchParams(search)
    const projectId = params.get(AUXILIARY_QUERY_PARAM_PROJECT_ID)
    const windowSlotRaw = params.get(AUXILIARY_QUERY_PARAM_WINDOW_SLOT)
    if (!projectId || windowSlotRaw === null) return MAIN_WINDOW_CONTEXT
    const windowSlot = Number(windowSlotRaw)
    if (!Number.isInteger(windowSlot)) return MAIN_WINDOW_CONTEXT
    return { kind: 'auxiliary', projectId, windowSlot }
}

export const getWindowContext = (): WindowContext => (typeof window === 'undefined' ? MAIN_WINDOW_CONTEXT : readWindowContext(window.location.search))
