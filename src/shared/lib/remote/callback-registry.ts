type Callback = (payload: unknown) => void

export type CallbackRegistry = {
    callbacks: Record<number, Callback>
    transformCallback: (callback: Callback, once?: boolean) => number
    runCallback: (id: number, payload: unknown) => void
    unregisterCallback: (id: number) => void
}

export const createCallbackRegistry = (): CallbackRegistry => {
    const callbacks: Record<number, Callback> = {}
    const onceIds = new Set<number>()
    let nextId = 1

    const transformCallback = (callback: Callback, once = false) => {
        const id = nextId
        nextId += 1
        callbacks[id] = callback
        if (once) onceIds.add(id)
        return id
    }

    const runCallback = (id: number, payload: unknown) => {
        const callback = callbacks[id]
        if (!callback) return
        callback(payload)
        if (onceIds.has(id)) {
            delete callbacks[id]
            onceIds.delete(id)
        }
    }

    const unregisterCallback = (id: number) => {
        delete callbacks[id]
        onceIds.delete(id)
    }

    return { callbacks, transformCallback, runCallback, unregisterCallback }
}
