import type { commands } from '@shared/api/bindings'

export type ResourceOperations = {
    renameEntry: (input: { from: string; to: string }) => Promise<null>
    deleteEntry: (path: Parameters<typeof commands.fileDelete>[0]) => Promise<null>
}

let operations: ResourceOperations | null = null

export const registerResourceOperations = (next: ResourceOperations) => {
    operations = next
}

export const renameWorkspaceEntry: ResourceOperations['renameEntry'] = async (input) => {
    if (!operations) throw new Error('Workspace file operations are not ready')
    await operations.renameEntry(input)
    return null
}

export const deleteWorkspaceEntry: ResourceOperations['deleteEntry'] = async (path) => {
    if (!operations) throw new Error('Workspace file operations are not ready')
    await operations.deleteEntry(path)
    return null
}
