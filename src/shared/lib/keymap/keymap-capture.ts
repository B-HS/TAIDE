type Listener = () => void

const listeners = new Set<Listener>()

let isCapturing = false

export const setKeymapCapturing = (value: boolean) => {
    if (value === isCapturing) return
    isCapturing = value
    for (const listener of listeners) listener()
}

export const subscribeKeymapCapturing = (listener: Listener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}

export const getKeymapCapturingSnapshot = () => isCapturing
