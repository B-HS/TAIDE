import { useSyncExternalStore } from 'react'
import { monaco } from '@shared/lib/monaco/setup'

type Listener = () => void

/**
 * Marker totals per severity, keyed by monaco's own `MarkerSeverity` rather than by a severity
 * vocabulary of this module's own: `features/problems/problem-severity.ts` already owns the
 * display-facing names, `shared` may not import from `features` (FSD), and naming them again here
 * would be the same monaco-severity mapping maintained in two places.
 */
export type MonacoMarkerCounts = Record<monaco.MarkerSeverity, number>

const listeners = new Set<Listener>()

/**
 * Every read of `monaco` here is deferred to call time — nothing in this module touches it while the
 * module body runs. Module-level access would make loading this file depend on monaco being fully
 * initialized at that moment, which is exactly what breaks under `bun test`: several suites stub
 * `@shared/lib/monaco/setup` with a partial fake, and `mock.module` is process-global, so whichever
 * stub happens to be installed when some other file first pulls this module in would decide whether
 * it evaluates at all.
 */
const markerSeverities = () => [monaco.MarkerSeverity.Hint, monaco.MarkerSeverity.Info, monaco.MarkerSeverity.Warning, monaco.MarkerSeverity.Error]

const emptyMarkerCounts = (): MonacoMarkerCounts => ({
    [monaco.MarkerSeverity.Hint]: 0,
    [monaco.MarkerSeverity.Info]: 0,
    [monaco.MarkerSeverity.Warning]: 0,
    [monaco.MarkerSeverity.Error]: 0,
})

let markersSnapshot: ReturnType<typeof monaco.editor.getModelMarkers> = []
let countsSnapshot: MonacoMarkerCounts | null = null
let monacoSubscription: { dispose: () => void } | null = null

const countMarkersBySeverity = (markers: typeof markersSnapshot) => {
    const counts = emptyMarkerCounts()
    for (const marker of markers) counts[marker.severity] += 1
    return counts
}

const areMarkerCountsEqual = (a: MonacoMarkerCounts, b: MonacoMarkerCounts) => markerSeverities().every((severity) => a[severity] === b[severity])

/**
 * Reads both tiers from the one `getModelMarkers({})` pass monaco makes us do (its marker service
 * has no severity filter, so counting cannot be cheaper than the full read). The counts object is
 * *kept* when the totals are unchanged: `useSyncExternalStore` bails out of a re-render only when
 * `getSnapshot` returns an identical reference, so replacing an equal object would re-render every
 * counts consumer on marker changes that do not move a single total — and, since the store notifies
 * synchronously during monaco's own event, would do it on the LSP indexing storm this two-tier split
 * exists to survive.
 */
const readSnapshots = () => {
    markersSnapshot = monaco.editor.getModelMarkers({})
    const nextCounts = countMarkersBySeverity(markersSnapshot)
    if (!countsSnapshot || !areMarkerCountsEqual(countsSnapshot, nextCounts)) countsSnapshot = nextCounts
}

const refreshSnapshot = () => {
    readSnapshots()
    listeners.forEach((listener) => listener())
}

/**
 * Subscribes to monaco's own `onDidChangeMarkers` only while at least one `useMonacoMarkers`
 * consumer is mounted (F7#17) — the previous version wired this up unconditionally at module load,
 * so the subscription (and the snapshot recompute it drives on every marker change anywhere in the
 * app) outlived every consumer for the app's entire lifetime, even when nothing ever rendered
 * `useMonacoMarkers`. Ref-counted by `listeners.size`: the first subscriber arms the real monaco
 * listener and takes a fresh snapshot (covering markers set before this subscriber mounted), the
 * last one unsubscribing tears it down. Exported (alongside {@link getMonacoMarkersSnapshot}) purely
 * so tests can exercise this ref-counted subscribe/unsubscribe lifecycle directly, without rendering
 * a component.
 *
 * Shared by both tiers ({@link useMonacoMarkers} and {@link useMonacoMarkerCounts}) so a counts-only
 * consumer keeps the ref count honest and arms the same single monaco listener.
 */
export const subscribeToMonacoMarkers = (listener: Listener) => {
    if (listeners.size === 0) {
        readSnapshots()
        monacoSubscription = monaco.editor.onDidChangeMarkers(refreshSnapshot)
    }
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
        if (listeners.size > 0) return
        monacoSubscription?.dispose()
        monacoSubscription = null
    }
}

export const getMonacoMarkersSnapshot = () => markersSnapshot

/**
 * Built on first read rather than at module load, for the deferral reason {@link markerSeverities}
 * documents. `useSyncExternalStore` calls this during the render *before* it subscribes, so the
 * empty table it creates here is what a consumer sees until the first snapshot read — and it is
 * cached, because returning a fresh object per call is the "getSnapshot should be cached" infinite
 * render loop.
 */
export const getMonacoMarkerCountsSnapshot = () => (countsSnapshot ??= emptyMarkerCounts())

/** Every marker in every model. For consumers that render the markers themselves (Problems panel). */
export const useMonacoMarkers = () => useSyncExternalStore(subscribeToMonacoMarkers, getMonacoMarkersSnapshot)

/**
 * Totals only, for consumers that display a number rather than the markers. The status bar is
 * mounted for the whole session and shows one count, so subscribing it to the full array made every
 * marker change anywhere in the app re-render it and re-scan every marker (research 3a H2); with
 * this tier it re-renders only when a total actually moves.
 */
export const useMonacoMarkerCounts = () => useSyncExternalStore(subscribeToMonacoMarkers, getMonacoMarkerCountsSnapshot)
