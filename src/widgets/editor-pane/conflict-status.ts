import type { StatusRow } from '@shared/api/bindings'

/**
 * Whether the open tab at `absolutePath` is an unresolved merge conflict, per `gitStatus.rows`.
 * `StatusRow.path` is repo-relative (see its doc comment); every file tab's path is absolute
 * (`explorer`/`git-panel` both open by `row.absPath`), so this must compare against `row.absPath`.
 */
export const isPathConflicted = (rows: StatusRow[], absolutePath: string) => rows.some((row) => row.absPath === absolutePath && row.isConflicted)
