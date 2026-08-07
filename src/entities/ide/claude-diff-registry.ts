type PendingClaudeDiff = {
    oldPath: string
    newContents: string
    tabName: string
}

const registry = new Map<string, PendingClaudeDiff>()

export const setPendingClaudeDiff = (requestId: string, pending: PendingClaudeDiff) => {
    registry.set(requestId, pending)
}

export const getPendingClaudeDiff = (requestId: string) => registry.get(requestId)

export const removePendingClaudeDiff = (requestId: string) => {
    registry.delete(requestId)
}
