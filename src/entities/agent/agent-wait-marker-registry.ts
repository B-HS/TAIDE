const pathToWaitMarkers = new Map<string, string[]>()

export const registerWaitMarker = (path: string, marker: string) => {
    const markers = pathToWaitMarkers.get(path) ?? []
    markers.push(marker)
    pathToWaitMarkers.set(path, markers)
}

/**
 * Removes and returns every wait marker registered for `path`. A path can accumulate more than
 * one marker (e.g. two overlapping `taide --wait` invocations targeting the same open tab), so
 * closing the tab must release all of them rather than only the most recently registered one.
 */
export const takeWaitMarkers = (path: string) => {
    const markers = pathToWaitMarkers.get(path)
    if (!markers) return []
    pathToWaitMarkers.delete(path)
    return markers
}
