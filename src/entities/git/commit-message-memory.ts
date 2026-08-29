import type { ProjectId } from '@shared/api/bindings'

/**
 * How many projects keep an unsent commit message. Entries are dropped by this cap and by a
 * successful commit; nothing observes a project being closed from here, so the cap is what keeps a
 * long multi-project session from holding drafts forever.
 */
export const COMMIT_MESSAGE_MEMORY_LIMIT = 8

/**
 * Per-project memory of the commit message the user has typed but not committed yet.
 *
 * The git panel only renders while the sidebar's git view is selected, so switching to the files or
 * search view unmounts `GitPanelContainer` and threw the message away with its component state.
 * Nothing above the sidebar remounts when the active project changes, on the other hand, so a
 * message typed for one project stayed on screen after switching to another and could be committed
 * into the wrong repository (audit §4-B C6). Keying the draft by `ProjectId` here closes both: the
 * message survives a view switch and follows the project it was written for.
 *
 * Insertion order doubles as recency (a rewrite deletes before setting), so the oldest entry is the
 * first key when the cap is exceeded.
 */
const messagesByProjectId = new Map<ProjectId, string>()

export const readCommitMessageDraft = (projectId: ProjectId) => messagesByProjectId.get(projectId) ?? ''

/**
 * Stores `message` for `projectId`, or forgets the project outright when the message is empty —
 * an emptied input is indistinguishable from never having typed one, and keeping it would let a
 * blank entry evict a real draft from another project through the cap.
 */
export const writeCommitMessageDraft = (projectId: ProjectId, message: string) => {
    messagesByProjectId.delete(projectId)
    if (message === '') return

    messagesByProjectId.set(projectId, message)
    while (messagesByProjectId.size > COMMIT_MESSAGE_MEMORY_LIMIT) {
        const oldest = messagesByProjectId.keys().next()
        if (oldest.done) return
        messagesByProjectId.delete(oldest.value)
    }
}
