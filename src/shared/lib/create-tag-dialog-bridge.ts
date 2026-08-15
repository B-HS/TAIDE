export type CreateTagRequest = { target: string }

type Listener = (request: CreateTagRequest) => void

let pendingRequest: CreateTagRequest | null = null
const listeners = new Set<Listener>()

export const requestOpenCreateTagDialog = (request: CreateTagRequest) => {
    if (listeners.size === 0) {
        pendingRequest = request
        return
    }
    for (const listener of listeners) listener(request)
}

export const subscribeOpenCreateTagDialog = (listener: Listener) => {
    listeners.add(listener)
    if (pendingRequest) {
        const request = pendingRequest
        pendingRequest = null
        listener(request)
    }
    return () => {
        listeners.delete(listener)
    }
}
