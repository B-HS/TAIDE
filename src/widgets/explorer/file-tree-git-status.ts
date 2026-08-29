import type { GitChangeKind, StatusRow } from '@shared/api/bindings'
import type { FileTreeGitStatus } from '@features/explorer/file-tree-row'
import { parentDirOf } from '@widgets/explorer/explorer-path'

type FileTreeGitDecoration = Exclude<FileTreeGitStatus, null | 'ignored'>

const CHANGE_KIND_DECORATION: Record<GitChangeKind, FileTreeGitDecoration> = {
    modified: 'modified',
    added: 'added',
    deleted: 'deleted',
    renamed: 'renamed',
    untracked: 'untracked',
    typeChange: 'modified',
    conflicted: 'conflicted',
}

/**
 * Merge order when one path carries several decorations (a staged and an unstaged change on the
 * same file, or a directory aggregating mixed children). Conflicts always surface first; "new"
 * states (added/untracked) outrank modified because the row visibly exists as a new file; deleted
 * ranks last since a deleted file has no row of its own — it only ever colors ancestor
 * directories, and any surviving sibling change is more actionable there.
 */
const DECORATION_PRIORITY: Record<FileTreeGitDecoration, number> = {
    conflicted: 6,
    added: 5,
    untracked: 4,
    renamed: 3,
    modified: 2,
    deleted: 1,
}

const resolveStatusRowDecoration = (row: StatusRow) => {
    if (row.isConflicted) return 'conflicted'
    const decorations = [row.staged, row.unstaged].filter((kind): kind is GitChangeKind => kind != null).map((kind) => CHANGE_KIND_DECORATION[kind])
    if (decorations.length === 0) return null
    return decorations.reduce((best, next) => (DECORATION_PRIORITY[next] > DECORATION_PRIORITY[best] ? next : best))
}

/**
 * Derives the explorer's per-path git decorations from `git_status` rows: each changed file keeps
 * its own state, and every ancestor directory below the project root inherits the highest-priority
 * state among its descendants (VS Code-style folder propagation). Keys are absolute paths so the
 * result can be looked up directly with `TreeRow.path`. The `startsWith(root + '/')` guard both
 * stops the walk at the project root and rejects prefix collisions between sibling directories
 * (`/a/b-c` is not inside `/a/b`).
 */
export const buildFileTreeGitStatusByPath = (rows: StatusRow[], projectRoot: string | null) => {
    const statusByPath = new Map<string, FileTreeGitDecoration>()
    if (!projectRoot) return statusByPath

    const mergeStatus = (path: string, decoration: FileTreeGitDecoration) => {
        const existing = statusByPath.get(path)
        if (existing && DECORATION_PRIORITY[existing] >= DECORATION_PRIORITY[decoration]) return
        statusByPath.set(path, decoration)
    }

    const rootPrefix = `${projectRoot}/`
    for (const row of rows) {
        const decoration = resolveStatusRowDecoration(row)
        if (!decoration) continue
        mergeStatus(row.absPath, decoration)
        for (let dir = parentDirOf(row.absPath); dir.startsWith(rootPrefix); dir = parentDirOf(dir)) mergeStatus(dir, decoration)
    }
    return statusByPath
}
