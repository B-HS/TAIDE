export type OpenWithOverride = 'editor' | 'preview'

type Listener = () => void

const overrideByPath = new Map<string, OpenWithOverride>()
const listeners = new Set<Listener>()

const notify = () => {
    for (const listener of listeners) listener()
}

export const setOpenWithOverride = (path: string, override: OpenWithOverride | null) => {
    if (override === null) overrideByPath.delete(path)
    else overrideByPath.set(path, override)
    notify()
}

export const getOpenWithOverride = (path: string): OpenWithOverride | null => overrideByPath.get(path) ?? null

export const subscribeOpenWithOverride = (listener: Listener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}
