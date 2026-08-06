import { useSyncExternalStore } from 'react'
import { monaco } from '@shared/lib/monaco/setup'

type Listener = () => void

const listeners = new Set<Listener>()

let snapshot: ReturnType<typeof monaco.editor.getModelMarkers> = monaco.editor.getModelMarkers({})

monaco.editor.onDidChangeMarkers(() => {
    snapshot = monaco.editor.getModelMarkers({})
    listeners.forEach((listener) => listener())
})

const subscribe = (listener: Listener) => {
    listeners.add(listener)
    return () => listeners.delete(listener)
}

const getSnapshot = () => snapshot

export const useMonacoMarkers = () => useSyncExternalStore(subscribe, getSnapshot)
