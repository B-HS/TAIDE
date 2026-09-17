import type { FileTreeRow } from '@features/explorer/file-tree-row'

const PATH_SEPARATOR = '/'

export const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

export const joinPath = (dir: string, name: string) => `${dir.endsWith(PATH_SEPARATOR) ? dir.slice(0, -1) : dir}${PATH_SEPARATOR}${name}`

/**
 * Which directory an explorer action (new file/folder, paste, "Open in Terminal") aims at, given
 * the row it was invoked on — the row itself when it is a directory, its parent when it is a file,
 * and the project root when there is no row.
 *
 * `rows` is taken as well so the answer stays a function of what the tree currently *holds*: a row
 * the user selected a moment ago can have been deleted since (or renamed away by the watcher), and
 * a directory path resolved off such a row would send the create straight back to a path that no
 * longer exists — `create_dir_all` in `domain::file::service::create_entry` then silently
 * resurrects the deleted directory (d-66 #13). A row that is no longer in the page is treated the
 * same as no row at all, which lands the action on the project root.
 */
export const resolveTargetDir = (row: FileTreeRow | null, rows: FileTreeRow[], projectRoot: string | null) => {
    if (!row || !rows.some((candidate) => candidate.path === row.path)) return projectRoot
    return row.kind === 'directory' ? row.path : parentDirOf(row.path)
}
