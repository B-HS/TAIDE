type WaitMarkerMap = Record<string, string[]>

const WAIT_MARKER_STORAGE_KEY = 'taide.agent.wait-markers'
const STARTUP_CLEANUP_DONE_KEY = 'taide.agent.wait-markers.startup-cleanup-done'

/**
 * Per-realm fallback for environments with no `localStorage` global (`bun test`, which has no DOM —
 * confirmed the app's other tests never rely on one either). Cross-window sharing doesn't apply
 * there anyway, since there is only ever one JS realm in that environment.
 */
let memoryFallbackMap: WaitMarkerMap = {}

const isWaitMarkerMap = (value: unknown): value is WaitMarkerMap => {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
    const candidate = value as Record<string, unknown>
    return Object.values(candidate).every((markers) => Array.isArray(markers) && markers.every((marker) => typeof marker === 'string'))
}

const readWaitMarkerMap = (): WaitMarkerMap => {
    if (typeof localStorage === 'undefined') return memoryFallbackMap

    const raw = localStorage.getItem(WAIT_MARKER_STORAGE_KEY)
    if (!raw) return {}
    try {
        const parsed: unknown = JSON.parse(raw)
        return isWaitMarkerMap(parsed) ? parsed : {}
    } catch {
        return {}
    }
}

const writeWaitMarkerMap = (map: WaitMarkerMap) => {
    if (typeof localStorage === 'undefined') {
        memoryFallbackMap = map
        return
    }
    localStorage.setItem(WAIT_MARKER_STORAGE_KEY, JSON.stringify(map))
}

/**
 * Backed by `localStorage`, not a module-scope `Map` — every TAIDE window (main and each auxiliary
 * `editor-<n>` window) loads the same `index.html` from the same origin, differing only in its URL
 * query string (`domain::window::commands::open_auxiliary_window`), so `localStorage` is one store
 * shared and synchronously readable across all of them. A plain in-memory `Map` is not: each window
 * is its own JS realm with its own module instances, so a marker registered by
 * `AgentExternalOpenProvider` (main-window-only) would be invisible to an auxiliary window's own
 * registry instance — and since a tab can move to an auxiliary window and be closed there
 * (`useCloseTab`), that mismatch would leave the marker registered forever, permanently blocking the
 * external CLI invocation that's waiting on it (contract #11).
 */
export const registerWaitMarker = (path: string, marker: string) => {
    const map = readWaitMarkerMap()
    writeWaitMarkerMap({ ...map, [path]: [...(map[path] ?? []), marker] })
}

/**
 * Removes and returns every wait marker registered for `path`. A path can accumulate more than
 * one marker (e.g. two overlapping `taide --wait` invocations targeting the same open tab), so
 * closing the tab must release all of them rather than only the most recently registered one.
 */
export const takeWaitMarkers = (path: string): string[] => {
    const map = readWaitMarkerMap()
    const markers = map[path]
    if (!markers) return []
    writeWaitMarkerMap(Object.fromEntries(Object.entries(map).filter(([key]) => key !== path)))
    return markers
}

/**
 * Discards every marker left over from a previous app run — a marker registered by a session that
 * then crashed or was force-quit before its tab closed (and so before `takeWaitMarkers` could ever
 * release it) would otherwise sit in `localStorage` forever, since nothing else ever revisits an
 * unclosed path. `sessionStorage` (unlike `localStorage`) does not survive an actual app relaunch
 * — a new OS process gets a fresh webview and a fresh `sessionStorage` — but it *does* survive an
 * in-window `location.reload()` (`window.reload` command), so this only clears once per real
 * relaunch, never on a reload of a window whose tabs (and their still-pending markers) are still
 * open. Call once, at the main window's startup — the only window `registerWaitMarker` ever runs
 * in (see that function's doc comment).
 */
export const clearStaleWaitMarkersOnStartup = () => {
    if (typeof sessionStorage === 'undefined') return
    if (sessionStorage.getItem(STARTUP_CLEANUP_DONE_KEY)) return
    writeWaitMarkerMap({})
    sessionStorage.setItem(STARTUP_CLEANUP_DONE_KEY, '1')
}
