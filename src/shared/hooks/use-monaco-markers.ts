import { useSyncExternalStore } from 'react'
import { monaco } from '@shared/lib/monaco/setup'

type Listener = () => void

const listeners = new Set<Listener>()

let snapshot: ReturnType<typeof monaco.editor.getModelMarkers> = []
let monacoSubscription: { dispose: () => void } | null = null

const refreshSnapshot = () => {
    snapshot = monaco.editor.getModelMarkers({})
    listeners.forEach((listener) => listener())
}

/**
 * Subscribes to monaco's own `onDidChangeMarkers` only while at least one `useMonacoMarkers`
 * consumer is mounted (F7#17) — the previous version wired this up unconditionally at module load,
 * so the subscription (and the snapshot recompute it drives on every marker change anywhere in the
 * app) outlived every consumer for the app's entire lifetime, even when nothing ever rendered
 * `useMonacoMarkers`. Ref-counted by `listeners.size`: the first subscriber arms the real monaco
 * listener and takes a fresh snapshot (covering markers set before this subscriber mounted), the
 * last one unsubscribing tears it down.
 */
/** Exported (alongside {@link getMonacoMarkersSnapshot}) purely so tests can exercise the ref-counted subscribe/unsubscribe lifecycle directly, without rendering a component. */
export const subscribeToMonacoMarkers = (listener: Listener) => {
    if (listeners.size === 0) {
        snapshot = monaco.editor.getModelMarkers({})
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

export const getMonacoMarkersSnapshot = () => snapshot

export const useMonacoMarkers = () => useSyncExternalStore(subscribeToMonacoMarkers, getMonacoMarkersSnapshot)
